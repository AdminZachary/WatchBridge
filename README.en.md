<div align="center">

<img src="public/watchbridge.svg" width="104" alt="WatchBridge" />

# WatchBridge

**Set up Apple Watch developer mode and install Watch Companion apps on your Watch — from Windows.**

[English](README.en.md) · [简体中文](README.md) · [한국어](README.ko.md) · [繁體中文](README.zh-TW.md) · [日本語](README.ja.md) · [Español](README.es.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Português](README.pt.md)

[![Build Windows](https://img.shields.io/github/actions/workflow/status/AdminZachary/iloader-for-windows/windows.yml?branch=main&label=windows&logo=github)](https://github.com/AdminZachary/iloader-for-windows/actions/workflows/windows.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11%20%C2%B7%20macOS%2011%2B-0078D4)
![Status](https://img.shields.io/badge/hardware%20validated-2026--11--12-brightgreen)
![Version](https://img.shields.io/badge/version-0.1.0--alpha.5-lightgrey)

</div>

> [!IMPORTANT]
> WatchBridge is an independent community derivative of [AndreySemjonov's iLoader fork](https://github.com/AndreySemjonov/iloader); the upstream project is [nab138/iLoader](https://github.com/nab138/iloader).
> It is **not an official version of Apple or of any upstream project, and implies no endorsement**. Please report problems to this repository, not upstream.

> [!NOTE]
> The full flow has been exercised on physical devices:
> developer-mode preparation → signing → phone install → watch install → app launch.

## What it is

Installing a self-signed iOS app on Windows (SideStore and the like) takes three things: an Apple ID signature, registering the target device as a development device, and enabling developer mode on the device. The official toolchain is Xcode, which only runs on macOS. This app fills that gap: it brings the whole workflow for enabling developer mode on the Watch and installing apps onto it to both Windows and macOS.

## Features

| | Feature | Details |
|---|---|---|
| 🛠 | **Developer mode preparation** | Pair the Watch → request the switch → read back the state. Only `action 0` is sent; **enabling, restarting and confirming are always done by you on the Watch** |
| 📦 | **Companion installation** | Verifies the IPA contains both the iPhone host and `Watch/*.app`, re-signs it, installs on the phone, then installs the Watch app |
| 📶 | **Wi-Fi / RSD** | Set up Wi-Fi once over USB, then install and renew wirelessly; developer-mode preparation still requires USB |
| ♻️ | **Automatic renewal** | Re-signs before the 7-day expiry, with tray controls and optional start at login |
| 🔐 | **Account and certificates** | Apple ID sign-in (device identity via anisette), certificate inspection/revocation, App ID management, keyring support |
| 🔗 | **Pairing file** | Import/export pairing files, repair Wi-Fi pairing |
| 🌍 | **Localized UI** | 28 languages, including Simplified and Traditional Chinese |

## How it works

```text
① Connect the paired iPhone over USB   ② Pair the Watch, confirm trust on it
   Select the device                      Request the Developer Mode switch
   ↓                                      ↓
   Enable it on the Watch → restart → confirm → click "Check again"
   ↓
③ Sign in with Apple ID → pick the original IPA → sign and install
   ↓
   Open the app on the Watch yourself and confirm it launches
```

> [!NOTE]
> The program does **not** enable developer mode, restart the Watch, or confirm trust for you. Trust records exist only in memory; restarting the program requires a new session.

## Requirements

**Windows**

- Windows 10 22H2 / Windows 11 (x86_64)
- **Apple Mobile Device Service**: shipped with [iTunes](https://www.apple.com/itunes/) or the Apple Devices app from the Microsoft Store. Without it neither USB nor Wi-Fi works
- WebView2 runtime (built into Windows 11)

**macOS**

- macOS 11 or newer (native on Apple Silicon, Intel works too)
- No driver install: the build uses the system `usbmuxd`
- No Xcode: the DMG runs as is; building from source only needs the Xcode command line tools

**Both**

- An iPhone already paired with the target Watch; preparation requires a cable

## Install and run

**Windows**: download `watchbridge-windows.zip` from [Releases](https://github.com/AdminZachary/iloader-for-windows/releases/latest), extract it and run `WatchBridge.exe`.

**macOS**: download the DMG for your architecture (`arm64` for Apple Silicon, `x86_64` for Intel) and drag `WatchBridge.app` into Applications. The first launch is blocked by Gatekeeper; allow it once in **System Settings → Privacy & Security → Open Anyway**.

The Windows archive contains:

| File | Contents |
|---|---|
| `WatchBridge.exe` | The application, **unsigned**; SmartScreen will warn on first run |
| `BUILD.json` | Source commit, whether the tree was dirty, lockfile hashes, toolchain versions, SHA-256 of the exe |
| `SHA256SUMS.txt` | Checksums of the files above |
| `LICENSES.zip` | Third-party license texts plus a dependency inventory (`inventory.json`) |

## Build from source

Dependencies and the full procedure are in [BUILDING.md](BUILDING.md). GitHub Actions compiles the app, runs the `iloader` and `isideload` Rust unit tests, and produces the artifact above.

```powershell
# Frontend only
bun install --frozen-lockfile --ignore-scripts
bun run build

# Full Windows build (tests + exe + checksum files)
.\scripts\Build-Windows.ps1 -BuildExecutable
```

Add `-SkipTests` for a faster local build without tests. The build must be native Windows + MSVC; cross-compiling is not supported.

## Compatibility and known limits

| Limit | Details |
|---|---|
| **One Watch** | If an iPhone has several paired Watches, preparation refuses instead of guessing. The forwarding protocol has no "target this Watch" parameter, and acting on the wrong Watch is worse than failing |
| **No Watch-only packages** | The original IPA must contain both the iPhone host and `Watch/*.app` |
| **App Group** | Apps that require a Watch App Group may still be affected by upstream skipping that association |
| **OS versions** | The Watch-side flow comes from xtool's experimental implementation; each iOS/watchOS combination needs device testing |
| **Unofficial tool** | No jailbreak, no bypassing Apple signatures, no forged certificates |

## FAQ

<details>
<summary><b><code>fail to connect to usbmuxd device socket: io error</code></b></summary>

On Windows, usbmuxd is provided by Apple Mobile Device Service and listens on `127.0.0.1:27015`. This error means nothing is answering on that port — usually the service is missing or stopped.

```powershell
Get-Service "Apple Mobile Device Service"      # should exist and be Running
Test-NetConnection 127.0.0.1 -Port 27015       # should be True
$env:USBMUXD_SOCKET_ADDRESS                    # should be empty
```

If the service does not exist, install iTunes or the Apple Devices app. If the address was overridden, remove that environment variable. The program **does not bundle** usbmuxd; it relies entirely on Apple's driver and service.
</details>

<details>
<summary><b>"Cannot read Watch information"</b></summary>

Older builds asked the iPhone's pairing registry for the Watch name before pairing; on recent iOS versions that query returns nothing. The current build establishes trust with the Watch first and reads its identity from the Watch's own session. If it still fails, open "Settings and logs → View logs" and check the `kind=` value of `watch step failed`: `timed out` means the Watch did not answer, while `NotFound` / `UnexpectedResponse` indicate a protocol difference.
</details>

<details>
<summary><b>"Multiple paired Watches found"</b></summary>

See the one-Watch limit above. Confirm the target Watch in the iPhone's Watch app; powering the other Watch off or moving it away does **not** remove it from the pairing registry.
</details>

## Contributing

- Contribution rules: [CONTRIBUTING.md](CONTRIBUTING.md)
- Upstream explicitly forbids AI-generated contributions, so **this repository never sends PRs upstream**. Keep changes in this fork
- Never attach Apple IDs, passwords, certificate private keys, pairing records or unredacted device logs

## License and credits

The project code is released under MIT ([LICENSE](LICENSE)); branding conditions are in [LICENSE-BRANDING](LICENSE-BRANDING).

> The signing library in the dependency chain (`apple-codesign-quick` from `apple-crates`) declares no license; its terms are still being clarified. See [docs/LICENSE_REVIEW.md](docs/LICENSE_REVIEW.md).

Sources and attribution: [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md), [vendor/isideload/PROVENANCE.md](vendor/isideload/PROVENANCE.md).

| Source | Contribution |
|---|---|
| [nab138/iLoader](https://github.com/nab138/iloader) | Original application, signing and installation pipeline |
| [AndreySemjonov/iloader](https://github.com/AndreySemjonov/iloader) | This repository's starting point: Wi-Fi/RSD and automatic renewal |
| [oacevedo/xtool](https://github.com/oacevedo/xtool/tree/feat/experimental-watchos) | Watch developer-mode preparation flow (rewritten natively in Rust here, no Python) |
| Data_C0re_ (Rzbck) | Watch Companion signing, registration and installation |
| idevice / isideload and their upstream authors | Device communication and signing implementation |

## Disclaimer

This project is not affiliated with Apple Inc. Apple, iPhone and Apple Watch are trademarks of Apple Inc.

For learning, research and personal development only; users are responsible for confirming the laws of their jurisdiction and the license terms of the apps they install. The authors accept no liability for device damage, data loss, account restrictions or certificate revocation.
