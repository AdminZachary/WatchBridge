import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { AppleID } from "./AppleID";
import { Device, DeviceInfo } from "./Device";
import { Modal } from "./components/Modal";
import { Certificates } from "./pages/Certificates";
import { Settings } from "./pages/Settings";
import { usePlatform } from "./PlatformContext";
import "./App.css";
import "./WatchBridge.css";

type WatchStatus = { name: string; paired: boolean; developerMode: boolean | null; switchRevealed: boolean };
type InstallUpdate = { updateType: "started" | "finished" | "failed"; stepId: string; extraDetails?: { message: string } };
type InstallState = "idle" | "validating" | "installing" | "done" | "failed";
const repo = "https://github.com/AdminZachary/WatchBridge";
const message = (error: unknown): string => typeof error === "object" && error !== null && "message" in error ? String(error.message) : String(error);

export default function App() {
  const [account, setAccount] = useState<string | null>(null);
  const [device, setDevice] = useState<DeviceInfo | null>(null);
  const [watch, setWatch] = useState<WatchStatus | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [watchError, setWatchError] = useState("");
  const [installError, setInstallError] = useState("");
  const [installState, setInstallState] = useState<InstallState>("idle");
  const [ipa, setIpa] = useState<string | null>(null);
  const [version, setVersion] = useState("");
  const [noKeyring, setNoKeyring] = useState(false);
  const [certificates, setCertificates] = useState(false);
  const [advanced, setAdvanced] = useState(false);
  const refreshDevices = useRef<(() => void) | null>(null);
  const inFlight = useRef(false);
  const { platform } = usePlatform();

  const checkKeyring = useCallback(async () => {
    setNoKeyring(!(await invoke<boolean>("keyring_available").catch(() => false)));
  }, []);
  useEffect(() => { void checkKeyring(); void getVersion().then(setVersion).catch(() => {}); }, [checkKeyring]);
  useEffect(() => { setWatch(null); setWatchError(""); setInstallState("idle"); setInstallError(""); }, [device?.udid, device?.id, device?.connectionType]);
  const registerRefresh = useCallback((fn?: () => void) => { refreshDevices.current = fn ?? null; }, []);

  const prepare = async (action: "check" | "pair" | "reveal") => {
    if (!device || inFlight.current) return;
    inFlight.current = true;
    setBusy(action);
    setWatchError("");
    // Discard a stale positive reading before a new device request.
    setWatch(null);
    try {
      const result = await invoke<WatchStatus>("prepare_watch", { action });
      setWatch(result);
    } catch (error) { setWatchError(message(error)); }
    finally { inFlight.current = false; setBusy(null); }
  };

  const install = async () => {
    if (!account || !device || !ipa || inFlight.current) return;
    inFlight.current = true;
    setBusy("install"); setInstallError(""); setInstallState("validating");
    let unlisten: UnlistenFn | undefined;
    try {
      unlisten = await listen<InstallUpdate>("operation_watch_sideload", ({ payload }) => {
        if (payload.stepId === "install" && payload.updateType === "started") setInstallState("installing");
      });
      await invoke("watch_sideload_operation", { appPath: ipa });
      setInstallState("done");
    } catch (error) { setInstallError(message(error)); setInstallState("failed"); }
    finally { unlisten?.(); inFlight.current = false; setBusy(null); }
  };

  const usb = device?.connectionType === "USB" && !device.networkAddress;
  const openLink = (url: string) => { void openUrl(url).catch(() => toast.error("无法打开链接")); };
  return (
    <main className="wb-app">
      <header className="wb-header">
        <div className="wb-brand"><span className="wb-mark" aria-hidden="true"><i /></span><div><strong>WatchBridge</strong><span>Apple Watch · Windows 助手</span></div></div>
        <div className="wb-header-right"><span className="wb-preview">v{version}</span><button onClick={() => openLink(repo)}>项目与源码 ↗</button></div>
      </header>
      <div className="wb-layout">
        <aside className="wb-sidebar">
          <div className="wb-intro"><span className="wb-eyebrow">从这里开始</span><h1>一台 Windows，<br />连接你的手表。</h1><p>通过已配对的 iPhone 准备开发者模式，并安装你的 Watch App。</p></div>
          <fieldset disabled={busy !== null} className="wb-card wb-device"><Device selectedDevice={device} setSelectedDevice={setDevice} registerRefresh={registerRefresh} /></fieldset>
          <fieldset disabled={busy !== null} className="wb-card"><AppleID loggedInAs={account} setLoggedInAs={setAccount} noKeyringAvailable={noKeyring} /></fieldset>
          <p className="wb-sidebar-note">准备开发者模式无需登录 Apple ID。签名安装时才需要登录。</p>
        </aside>
        <div className="wb-main">
          <section className="wb-card wb-step">
            <div className="wb-step-heading"><span className="wb-step-number">01</span><div><h2>准备开发者模式</h2><p>显示开关，随后在手表上完成开启。</p></div><span className={`wb-status ${watch?.developerMode === true ? "is-ready" : ""}`}>{watch?.developerMode === true ? "已确认开启" : "等待检查"}</span></div>
            <ol className="wb-instructions"><li>用 USB 连接已配对的 iPhone，解锁手机并信任电脑。</li><li>点击“配对手表”，保持手表解锁、亮屏，确认手表上的信任提示。</li><li>显示开关后，前往手表“设置 → 隐私与安全性 → 开发者模式”，开启、重启并确认。</li></ol>
            <div className="wb-actions">
              <button className="wb-primary" disabled={!usb || busy !== null} onClick={() => void prepare("pair")}>{busy === "pair" ? "请在手表上确认信任…" : "配对手表"}</button>
              <button disabled={!usb || !watch?.paired || busy !== null || watch.developerMode === true} onClick={() => void prepare("reveal")}>{busy === "reveal" ? "正在请求…" : "显示开发者模式开关"}</button>
              <button disabled={!usb || busy !== null} onClick={() => void prepare("check")}>{busy === "check" ? "检查中…" : "重新检查"}</button>
            </div>
            {!usb && <p className="wb-note">请先在左侧选择 iPhone 的 USB 连接。</p>}
            {watch && <div className="wb-result" role="status"><strong>{watch.name}</strong><p>{watch.developerMode === true ? "手表已返回开发者模式开启状态。可以继续导入 IPA。" : watch.switchRevealed ? "显示开关请求已确认。请在手表上开启、重启并确认，然后点击“重新检查”。" : watch.paired ? "已建立信任。若手表设置里没有开发者模式，请点击“显示开发者模式开关”。" : "发现配对手表。请先点击“配对手表”建立本次会话。"}</p></div>}
            {watchError && <p className="wb-error" role="alert">{watchError}</p>}
            <p className="wb-note">程序只请求显示开关；开启、重启和最终确认由你在手表上完成。重启本程序后需重新建立手表会话。</p>
          </section>
          <section className="wb-card wb-step">
            <div className="wb-step-heading"><span className="wb-step-number">02</span><div><h2>安装 Watch App</h2><p>导入包含 iPhone 宿主与 Watch Companion 的原始 IPA。</p></div></div>
            <div className="wb-file"><span className="wb-file-icon" aria-hidden="true">↥</span><div><strong>{ipa ? ipa.split(/[\\/]/).pop() : "选择你的 IPA 文件"}</strong><p>{ipa ? "文件已选择，尚未安装" : "第一版支持嵌入式 Watch Companion；暂不支持独立 Watch-only 包。"}</p></div><button disabled={busy !== null} onClick={async () => { try { const path = await openFileDialog({ multiple: false, filters: [{ name: "应用安装包", extensions: ["ipa"] }] }); if (typeof path === "string") { setIpa(path); setInstallState("idle"); setInstallError(""); } } catch (error) { setInstallError(message(error)); } }}>{ipa ? "更换文件" : "选择 IPA"}</button></div>
            <div className="wb-install-footer"><div><strong>{installState === "validating" ? "正在检查 IPA…" : installState === "installing" ? "正在签名并安装，请保持设备连接…" : "准备好后开始安装"}</strong><p>签名与安装需要 Apple ID。请保持手机和手表解锁、靠近。</p></div><button className="wb-primary" disabled={!ipa || !account || !device || busy !== null} onClick={() => void install()}>{busy === "install" ? "安装进行中…" : "签名并安装"}</button></div>
            {!account && <p className="wb-note">请先在左侧登录 Apple ID。</p>}
            {installState === "done" && <div className="wb-result" role="status"><strong>安装流程已完成</strong><p>请在 Apple Watch 上手动打开应用，确认能够正常运行。安装完成不代表启动已经验证。</p></div>}
            {installError && <p className="wb-error" role="alert">{installError}</p>}
            <details className="wb-details"><summary>USB 安装失败或需要网络连接？</summary><p>保留了上游的 Wi-Fi / RSD 安装能力。先使用 USB 设备卡片中的 Wi-Fi 设置，再选择 Network 连接安装；第一步的开发者模式准备仍需 USB。不同系统版本需要实机验证。</p></details>
          </section>
          <div className="wb-tools"><button disabled={!account || busy !== null} onClick={() => setCertificates(true)}>管理签名证书</button><button disabled={busy !== null} onClick={() => setAdvanced(!advanced)}>{advanced ? "收起设置与日志" : "设置与日志"}</button></div>
          {advanced && <Settings ensureSelectedDevice={() => Boolean(device)} setSelectedDevice={setDevice} platform={platform} shortcutLabel={(mac, windows) => platform === "mac" ? mac : windows} checkKeyring={checkKeyring} />}
          <footer className="wb-attribution">独立社区项目，基于 <button onClick={() => openLink("https://github.com/nab138/iloader")}>nab138 / iLoader</button>、<button onClick={() => openLink("https://github.com/AndreySemjonov/iloader")}>Andrey 的 fork</button>、Rzbck 的 Watch 安装贡献及 <button onClick={() => openLink("https://github.com/oacevedo/xtool/tree/feat/experimental-watchos")}>oacevedo / xtool</button>。不代表上游官方版本。</footer>
        </div>
      </div>
      <Modal isOpen={certificates} close={() => setCertificates(false)}><Certificates /></Modal>
    </main>
  );
}
