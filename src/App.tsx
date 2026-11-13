import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { Trans, useTranslation } from "react-i18next";
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
  const { t } = useTranslation();

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
  const openLink = (url: string) => { void openUrl(url).catch(() => toast.error(t("wb.open_link_failed"))); };
  return (
    <main className="wb-app">
      <header className="wb-header">
        <div className="wb-brand"><span className="wb-mark" aria-hidden="true"><i /></span><div><strong>WatchBridge</strong><span>{t("wb.header_sub")}</span></div></div>
        <div className="wb-header-right"><span className="wb-preview">v{version}</span><button onClick={() => openLink(repo)}>{t("wb.repo_link")}</button></div>
      </header>
      <div className="wb-layout">
        <aside className="wb-sidebar">
          <div className="wb-intro"><span className="wb-eyebrow">{t("wb.eyebrow")}</span><h1><Trans i18nKey="wb.headline" /></h1><p>{t("wb.intro")}</p></div>
          <fieldset disabled={busy !== null} className="wb-card wb-device"><Device selectedDevice={device} setSelectedDevice={setDevice} registerRefresh={registerRefresh} /></fieldset>
          <fieldset disabled={busy !== null} className="wb-card"><AppleID loggedInAs={account} setLoggedInAs={setAccount} noKeyringAvailable={noKeyring} /></fieldset>
          <p className="wb-sidebar-note">{t("wb.sidebar_note")}</p>
        </aside>
        <div className="wb-main">
          <section className="wb-card wb-step">
            <div className="wb-step-heading"><span className="wb-step-number">01</span><div><h2>{t("wb.s1_title")}</h2><p>{t("wb.s1_sub")}</p></div><span className={`wb-status ${watch?.developerMode === true ? "is-ready" : ""}`}>{watch?.developerMode === true ? t("wb.status_on") : t("wb.status_wait")}</span></div>
            <ol className="wb-instructions"><li>{t("wb.s1_step1")}</li><li>{t("wb.s1_step2")}</li><li>{t("wb.s1_step3")}</li></ol>
            <div className="wb-actions">
              <button className="wb-primary" disabled={!usb || busy !== null} onClick={() => void prepare("pair")}>{busy === "pair" ? t("wb.btn_pair_busy") : t("wb.btn_pair")}</button>
              <button disabled={!usb || !watch?.paired || busy !== null || watch.developerMode === true} onClick={() => void prepare("reveal")}>{busy === "reveal" ? t("wb.btn_reveal_busy") : t("wb.btn_reveal")}</button>
              <button disabled={!usb || busy !== null} onClick={() => void prepare("check")}>{busy === "check" ? t("wb.btn_check_busy") : t("wb.btn_check")}</button>
            </div>
            {!usb && <p className="wb-note">{t("wb.note_usb")}</p>}
            {watch && <div className="wb-result" role="status"><strong>{watch.name}</strong><p>{watch.developerMode === true ? t("wb.res_on") : watch.switchRevealed ? t("wb.res_revealed") : watch.paired ? t("wb.res_paired") : t("wb.res_found")}</p></div>}
            {watchError && <p className="wb-error" role="alert">{watchError}</p>}
            <p className="wb-note">{t("wb.note_program")}</p>
          </section>
          <section className="wb-card wb-step">
            <div className="wb-step-heading"><span className="wb-step-number">02</span><div><h2>{t("wb.s2_title")}</h2><p>{t("wb.s2_sub")}</p></div></div>
            <div className="wb-file"><span className="wb-file-icon" aria-hidden="true">↥</span><div><strong>{ipa ? ipa.split(/[\\/]/).pop() : t("wb.file_pick")}</strong><p>{ipa ? t("wb.file_chosen") : t("wb.file_hint")}</p></div><button disabled={busy !== null} onClick={async () => { try { const path = await openFileDialog({ multiple: false, filters: [{ name: t("wb.file_filter"), extensions: ["ipa"] }] }); if (typeof path === "string") { setIpa(path); setInstallState("idle"); setInstallError(""); } } catch (error) { setInstallError(message(error)); } }}>{ipa ? t("wb.btn_change") : t("wb.btn_pick")}</button></div>
            <div className="wb-install-footer"><div><strong>{installState === "validating" ? t("wb.inst_validating") : installState === "installing" ? t("wb.inst_installing") : t("wb.inst_ready")}</strong><p>{t("wb.inst_note")}</p></div><button className="wb-primary" disabled={!ipa || !account || !device || busy !== null} onClick={() => void install()}>{busy === "install" ? t("wb.btn_install_busy") : t("wb.btn_install")}</button></div>
            {!account && <p className="wb-note">{t("wb.note_login")}</p>}
            {installState === "done" && <div className="wb-result" role="status"><strong>{t("wb.done_title")}</strong><p>{t("wb.done_body")}</p></div>}
            {installError && <p className="wb-error" role="alert">{installError}</p>}
            <details className="wb-details"><summary>{t("wb.usb_fail_summary")}</summary><p>{t("wb.usb_fail_body")}</p></details>
          </section>
          <div className="wb-tools"><button disabled={!account || busy !== null} onClick={() => setCertificates(true)}>{t("wb.btn_certs")}</button><button disabled={busy !== null} onClick={() => setAdvanced(!advanced)}>{advanced ? t("wb.btn_settings_hide") : t("wb.btn_settings")}</button></div>
          {advanced && <Settings ensureSelectedDevice={() => Boolean(device)} setSelectedDevice={setDevice} platform={platform} shortcutLabel={(mac, windows) => platform === "mac" ? mac : windows} checkKeyring={checkKeyring} />}
          <footer className="wb-attribution"><Trans i18nKey="wb.footer" components={{ l1: <button onClick={() => openLink("https://github.com/nab138/iloader")} />, l2: <button onClick={() => openLink("https://github.com/AndreySemjonov/iloader")} />, l3: <button onClick={() => openLink("https://github.com/oacevedo/xtool/tree/feat/experimental-watchos")} /> }} /></footer>
        </div>
      </div>
      <Modal isOpen={certificates} close={() => setCertificates(false)}><Certificates /></Modal>
    </main>
  );
}
