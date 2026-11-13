<div align="center">

<img src="public/watchbridge.svg" width="104" alt="WatchBridge" />

# WatchBridge

**在 Windows 上準備 Apple Watch 開發者模式，並把帶 Watch Companion 的 App 裝到手錶上。**

[English](README.md) · [简体中文](README.zh-CN.md) · [한국어](README.ko.md) · [繁體中文](README.zh-TW.md) · [日本語](README.ja.md) · [Español](README.es.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Português](README.pt.md)

[![Build Windows](https://img.shields.io/github/actions/workflow/status/AdminZachary/WatchBridge/windows.yml?branch=main&label=windows&logo=github)](https://github.com/AdminZachary/WatchBridge/actions/workflows/windows.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11%20%C2%B7%20macOS%2011%2B-0078D4)
![Status](https://img.shields.io/badge/hardware%20validated-2026--11--12-brightgreen)
![Version](https://img.shields.io/badge/version-0.1.0-blue)

![Downloads over time](docs/downloads.svg)

</div>

> [!IMPORTANT]
> WatchBridge 是 [AndreySemjonov 的 iLoader fork](https://github.com/AndreySemjonov/iloader) 的獨立社群衍生版本，上游為 [nab138/iLoader](https://github.com/nab138/iloader)。
> 它**不代表 Apple 或任何上游專案的官方版本，也不暗示其背書**。發生問題請回報到本儲存庫，不要打擾上游。

> [!NOTE]
> 已在實機裝置上跑通完整流程：
> 開發者模式準備 → 簽章 → 手機安裝 → 手錶安裝 → App 啟動。

## 這是什麼

在 Windows 上安裝自簽 iOS App（SideStore 之類）需要三件事：Apple ID 簽章、將目標裝置註冊為開發裝置、裝置開啟開發者模式。官方工具鏈是 Xcode，只有 macOS 有。本程式補上這一塊，把手錶開發者模式開啟與安裝的完整流程帶到 Windows 與 macOS。

## 功能

| | 功能 | 說明 |
|---|---|---|
| 🛠 | **開發者模式準備** | 配對手錶 → 請求顯示開關 → 讀回狀態。只送出 `action 0`，**開啟／重新啟動／確認始終由你在手錶上完成** |
| 📦 | **Companion 安裝** | 檢查 IPA 同時包含 iPhone 宿主與 `Watch/*.app`，重新簽章後裝到手機，再把 Watch App 裝到手錶 |
| 📶 | **Wi-Fi / RSD** | USB 設定一次 Wi-Fi，之後可無線安裝與續簽；開發者模式準備仍需 USB |
| ♻️ | **自動續簽** | 7 天簽章到期前自動重簽，附系統匣控制與可選開機啟動 |
| 🔐 | **帳號與憑證** | Apple ID 登入（anisette 提供裝置身分）、憑證檢視／撤銷、App ID 管理、keyring 支援 |
| 🔗 | **配對檔案** | 匯入／匯出 pairing file，修復 Wi-Fi 配對 |
| 🌍 | **多語系介面** | 28 種語言，含簡體／繁體中文 |

## 使用流程

```text
① 用 USB 連接已配對的 iPhone      ② 配對手錶，在手錶上確認信任
   選擇裝置                          請求顯示「開發者模式」開關
   ↓                                 ↓
   在手錶上手動開啟 → 重新啟動 → 確認 → 點「重新檢查」
   ↓
③ 登入 Apple ID → 選擇原始 IPA → 簽章並安裝
   ↓
   在手錶上手動開啟 App，確認能夠啟動
```

> [!NOTE]
> 程式**不會**自動開啟開發者模式、不會重新啟動手錶、不會代替你確認信任提示。手錶信任記錄只存在記憶體中，結束程式後需要重新建立工作階段。

## 系統需求

**Windows**

- Windows 10 22H2 / Windows 11（x86_64）
- **Apple Mobile Device Service**：來自 [iTunes](https://www.apple.com/itunes/) 或 [Microsoft Store 的 Apple Devices App](https://apps.microsoft.com/detail/9np83lwlpz9k)。沒有它，USB 與 Wi-Fi 兩條路都不通
- WebView2 執行階段（Windows 11 已內建）

**macOS**

- macOS 11 或更新（Apple Silicon 原生，Intel 亦可）
- 無需安裝驅動程式：直接使用系統內建的 `usbmuxd`
- 無需 Xcode：DMG 開箱即用；自行建置也只需要 Xcode Command Line Tools

**通用**

- iPhone 已與目標手錶配對；準備階段需要傳輸線

## 安裝與執行

**Windows**：從 [Releases](https://github.com/AdminZachary/WatchBridge/releases/latest) 下載 `watchbridge-windows.zip`，解壓縮後直接執行 `WatchBridge.exe`。

**macOS**：下載對應架構的 DMG（Apple Silicon 用 `arm64`，Intel 用 `x86_64`），把 `WatchBridge.app` 拖進「應用程式」。首次開啟會被 Gatekeeper 攔截，到「系統設定 → 隱私權與安全性」點一次「仍要打開」即可。

Windows 壓縮檔包含：

## 從原始碼建置

相依性與完整流程見 [BUILDING.md](BUILDING.md)。GitHub Actions 會編譯、執行 `iloader` 與 `isideload` 的 Rust 單元測試，並產出上面那份 artifact。

```powershell
# 只建置前端
bun install --frozen-lockfile --ignore-scripts
bun run build

# 完整 Windows 建置（含單元測試 + exe + 校驗檔）
.\scripts\Build-Windows.ps1 -BuildExecutable
```

本機快速建置可加 `-SkipTests` 跳過單元測試。建置必須是 Windows + MSVC 原生，不做交叉編譯。

## 相容性與已知限制

| 限制 | 說明 |
|---|---|
| **單一手錶** | 一台 iPhone 配對多支手錶時，準備流程會明確拒絕而不是猜測。轉發協定沒有「指定目標手錶」的參數，選錯比失敗更糟 |
| **不支援 Watch-only 套件** | 需要同時包含 iPhone 宿主與 `Watch/*.app` 的原始 IPA |
| **App Group** | 需要 Watch App Group 的 App 可能仍受上游略過關聯行為的影響 |
| **系統版本** | 手錶端流程源自 xtool 的實驗性實作，不同 iOS/watchOS 組合需要實機驗證 |
| **非官方工具** | 不越獄、不繞過 Apple 簽章、不偽造憑證 |

## 常見問題

<details>
<summary><b><code>fail to connect to usbmuxd device socket: io error</code></b></summary>

Windows 上的 usbmuxd 由 Apple Mobile Device Service 提供，監聽 `127.0.0.1:27015`。出現這個錯誤代表該連接埠沒有回應——通常是服務未安裝或未啟動。

```powershell
Get-Service "Apple Mobile Device Service"      # 應存在且為 Running
Test-NetConnection 127.0.0.1 -Port 27015       # 應為 True
$env:USBMUXD_SOCKET_ADDRESS                    # 應為空
```

服務不存在 → 安裝 iTunes 或 Apple Devices App；位址被改過 → 刪除該環境變數。程式**不內建** usbmuxd，完全依賴 Apple 的驅動程式與服務。
</details>

<details>
<summary><b>提示「無法讀取手錶資訊」</b></summary>

舊版在配對前向 iPhone 的配對登錄檔查詢手錶名稱，在較新的 iOS 上該查詢不會回傳。新版改為先與手錶建立信任，再從手錶自己的工作階段讀取身分。若仍失敗，開啟「設定與記錄 → 檢視記錄」，查看 `watch step failed` 的 `kind=` 值：`timed out` 是手錶沒有回應，`NotFound` / `UnexpectedResponse` 是協定差異。
</details>

<details>
<summary><b>提示「發現多支配對手錶」</b></summary>

見上面的單一手錶限制。請在 iPhone 的 Watch App 中確認目標手錶；把另一支手錶關機或放遠**不會**讓它從配對登錄檔中消失。
</details>

## 參與貢獻

- 貢獻規則見 [CONTRIBUTING.md](CONTRIBUTING.md)
- 上游明確禁止 AI 生成的貢獻，**本儲存庫不會向上游提交 PR**，請把變更留在這個 fork
- 請勿提交 Apple ID、密碼、憑證私鑰、配對記錄或未去識別化的裝置記錄

## 授權與致謝

本專案程式碼以 MIT 釋出（[LICENSE](LICENSE)），品牌使用條件見 [LICENSE-BRANDING](LICENSE-BRANDING)。

> 相依鏈中的簽章函式庫（`apple-crates` 的 `apple-codesign-quick`）未宣告授權，其條款仍待釐清。詳見 [docs/LICENSE_REVIEW.md](docs/LICENSE_REVIEW.md)。

來源與歸屬詳見 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)、[vendor/isideload/PROVENANCE.md](vendor/isideload/PROVENANCE.md)：

| 來源 | 貢獻 |
|---|---|
| [nab138/iLoader](https://github.com/nab138/iloader) | 原始 App、簽章與安裝流程 |
| [AndreySemjonov/iloader](https://github.com/AndreySemjonov/iloader) | 本儲存庫的起始分支：Wi-Fi/RSD 與自動續簽 |
| [oacevedo/xtool](https://github.com/oacevedo/xtool/tree/feat/experimental-watchos) | 手錶開發者模式準備流程（本儲存庫以 Rust 原生重寫，不含 Python） |
| Data_C0re_ (Rzbck) | Watch Companion 的簽章、註冊與安裝 |
| idevice / isideload 及上游作者 | 裝置通訊與簽章實作 |

## 免責聲明

本專案與 Apple Inc. 無任何關聯。Apple、iPhone、Apple Watch 是 Apple Inc. 的商標。

僅供學習、研究與個人開發使用；使用者需自行確認其所在地法律與所安裝 App 的授權條款。作者不對裝置損壞、資料遺失、帳號風控或憑證撤銷承擔責任。
