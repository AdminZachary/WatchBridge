<div align="center">

<img src="public/watchbridge.svg" width="104" alt="WatchBridge" />

# WatchBridge

**在 Windows 上准备 Apple Watch 开发者模式，并把带 Watch Companion 的应用装到手表上。**

[English](README.md) · [简体中文](README.zh-CN.md) · [한국어](README.ko.md) · [繁體中文](README.zh-TW.md) · [日本語](README.ja.md) · [Español](README.es.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Português](README.pt.md)

[![Build Windows](https://img.shields.io/github/actions/workflow/status/AdminZachary/WatchBridge/windows.yml?branch=main&label=windows&logo=github)](https://github.com/AdminZachary/WatchBridge/actions/workflows/windows.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11%20%C2%B7%20macOS%2011%2B-0078D4)
![Status](https://img.shields.io/badge/hardware%20validated-2026--11--12-brightgreen)
![Version](https://img.shields.io/badge/version-0.1.1-blue)


</div>

> [!IMPORTANT]
> WatchBridge 是 [AndreySemjonov 的 iLoader fork](https://github.com/AndreySemjonov/iloader) 的独立社区衍生版本，上游为 [nab138/iLoader](https://github.com/nab138/iloader)。
> 它**不代表 Apple 或任何上游项目的官方版本，也不暗示其背书**。出现问题时请报给本仓库，不要打扰上游。

> [!NOTE]
> 已在实机设备上跑通全流程：
> 开发者模式准备 → 签名 → 手机安装 → 手表安装 → 应用启动。

## 这是什么

Windows 上装自签 iOS 应用（SideStore 之类）需要三件事：Apple ID 签名、将目标设备注册为开发设备、设备开启开发者模式。官方工具链是 Xcode，只有 macOS 有。此 APP 解决这个问题，实现手表开发者模式开启与安装的完整流程，Windows 与 macOS 皆可。

## 功能

| | 功能 | 说明 |
|---|---|---|
| 🛠 | **开发者模式准备** | 配对手表 → 请求显示开关 → 读回状态。只发送 `action 0`，**开启/重启/确认始终由你在手表上完成** |
| 📦 | **Companion 安装** | 校验 IPA 同时包含 iPhone 宿主与 `Watch/*.app`，重签后装到手机，再把 Watch 应用装到手表 |
| 📶 | **Wi-Fi / RSD** | USB 配一次 Wi-Fi，之后可无线安装与续签；开发者模式准备仍需 USB |
| ♻️ | **自动续签** | 7 天签名到期前自动重签，带托盘控制与可选开机启动 |
| 🔐 | **账号与证书** | Apple ID 登录（anisette 提供设备身份）、证书查看/吊销、App ID 管理、keyring 支持 |
| 🔗 | **配对文件** | 导入/导出 pairing file，修复 Wi-Fi 配对 |
| 🌍 | **多语言界面** | 28 种语言，含简体/繁体中文 |

## 使用流程

```text
① USB 连接已配对的 iPhone          ② 配对手表，手表上确认信任
   选择设备                           请求显示「开发者模式」开关
   ↓                                  ↓
   在手表上手动开启 → 重启 → 确认 → 点「重新检查」
   ↓
③ 登录 Apple ID → 选择原始 IPA → 签名并安装
   ↓
   在手表上手动打开应用，确认能够启动
```

> [!NOTE]
> 程序**不会**自动开启开发者模式、不会重启手表、不会代替你确认信任提示。手表信任记录只存在内存中，退出程序后需要重新建立会话。

## 系统要求

**Windows**

- Windows 10 22H2 / Windows 11（x86_64）
- **Apple Mobile Device Service**：来自 [iTunes](https://www.apple.com/itunes/) 或 [Microsoft Store 的 Apple Devices 应用](https://apps.microsoft.com/detail/9np83lwlpz9k)。没有它，USB 和 Wi-Fi 两条路都不通
- WebView2 运行时（Windows 11 已内置）

**macOS**

- macOS 11 或更新（Apple Silicon 原生，Intel 亦可）
- 无需安装驱动：直接使用系统自带的 `usbmuxd`
- 无需 Xcode：DMG 开箱即用；自行构建也只需要 Xcode Command Line Tools

**通用**

- iPhone 已与目标手表配对；准备阶段需要数据线

## 安装与运行

**Windows**：从 [Releases](https://github.com/AdminZachary/WatchBridge/releases/latest) 下载 `watchbridge-windows.zip`，解压后直接运行 `WatchBridge.exe`。

**macOS**：下载对应架构的 DMG（Apple Silicon 用 `arm64`，Intel 用 `x86_64`），把 `WatchBridge.app` 拖进「应用程序」。首次打开会被 Gatekeeper 拦截，到「系统设置 → 隐私与安全性」点一次「仍要打开」即可。

Windows 压缩包里包含：

| 文件 | 内容 |
|---|---|
| `WatchBridge.exe` | 主程序，**未签名**，首次运行会有 SmartScreen 提示 |
| `BUILD.json` | 源码提交、构建是否脏、锁文件哈希、工具链版本、exe 的 SHA-256 |
| `SHA256SUMS.txt` | 上述文件的校验和 |
| `LICENSES.zip` | 第三方许可证原文 + 依赖清单（`inventory.json`） |

## 从源码构建

依赖与完整流程见 [BUILDING.md](BUILDING.md)。GitHub Actions 会编译、运行 `iloader` 与 `isideload` 的 Rust 单测，并产出上面那份 artifact。

```powershell
# 只构建前端
bun install --frozen-lockfile --ignore-scripts
bun run build

# 完整 Windows 构建（含单测 + exe + 校验文件）
.\scripts\Build-Windows.ps1 -BuildExecutable
```

本地快速构建可加 `-SkipTests` 跳过单测。构建必须是 Windows + MSVC 原生，不做交叉编译。

## 兼容性与已知限制

| 限制 | 说明 |
|---|---|
| **单块手表** | 一台 iPhone 配对了多块手表时，准备流程会明确拒绝而不是猜。转发协议没有"指定目标手表"的参数，选错比失败更糟 |
| **不支持 Watch-only 包** | 需要同时包含 iPhone 宿主和 `Watch/*.app` 的原始 IPA |
| **App Group** | 需要 Watch App Group 的应用可能仍受上游跳过关联行为的影响 |
| **系统版本** | 手表侧流程源自 xtool 的实验性实现，不同 iOS/watchOS 组合需要实机验证 |
| **非官方工具** | 不越狱、不绕过苹果签名、不伪造证书 |

## 常见问题

<details>
<summary><b><code>fail to connect to usbmuxd device socket: io error</code></b></summary>

Windows 上的 usbmuxd 由 Apple Mobile Device Service 提供，监听 `127.0.0.1:27015`。报这个错说明端口上没人应答——通常是服务没安装或没启动。

```powershell
Get-Service "Apple Mobile Device Service"      # 应存在且为 Running
Test-NetConnection 127.0.0.1 -Port 27015       # 应为 True
$env:USBMUXD_SOCKET_ADDRESS                    # 应为空
```

服务不存在 → 安装 iTunes 或 Apple Devices 应用；服务被改过地址 → 删掉这个环境变量。程序**不打包** usbmuxd，完全依赖苹果的驱动与服务。
</details>

<details>
<summary><b>提示「无法读取手表信息」</b></summary>

旧版本在配对前向 iPhone 的配对注册表查询表名，在较新的 iOS 上该查询不返回。新版本改为先与手表建立信任、再从手表自己的会话读取身份。若仍失败，打开「设置与日志 → 查看日志」，查看 `watch step failed` 的 `kind=` 值：`timed out` 是手表没应答，`NotFound` / `UnexpectedResponse` 是协议差异。
</details>

<details>
<summary><b>提示「发现多块配对手表」</b></summary>

见上面的单表限制。请在 iPhone 的 Watch App 中确认目标手表；把另一块表关机或放远**不会**让它从配对注册表里消失。
</details>

<details>
<summary><b>登录 Apple ID 报错 <code>-23782: A Mac with macOS version 10.13 or later is required…</code></b></summary>

Apple 服务端会校验登录请求里声明的客户端身份（`X-Mme-Client-Info`）对应的系统版本，过旧即拒绝；而公共 anisette 服务器目前大多仍上报 `Mac OS X;10.12.6`（低于 10.13），因此登录在第一步就被 Apple 挡下。这与本机无关——Windows/macOS 上都会遇到。

新版 WatchBridge 在发起登录时强制携带一套已验证的现代 Mac 身份（与上游 iLoader 相同的做法），不再依赖服务器上报的版本声明。请更新到包含此修复的构建。若仍在旧版上遇到该错误，可先在「设置 → Anisette 服务器」里换一个已上报新版身份的服务器，或重置 Anisette 状态后重试。
</details>

## 参与贡献

- 贡献规则见 [CONTRIBUTING.md](CONTRIBUTING.md)
- 上游明确禁止 AI 生成贡献，**本仓库不会向上游提交 PR**，请把改动留在本 fork
- 请勿提交 Apple ID、密码、证书私钥、配对记录或未脱敏的设备日志

## 许可与致谢

本项目代码以 MIT 发布（[LICENSE](LICENSE)），品牌使用条件见 [LICENSE-BRANDING](LICENSE-BRANDING)。

> 依赖链中的签名库（`apple-crates` 的 `apple-codesign-quick`）未声明许可证，其许可仍待澄清。详见 [docs/LICENSE_REVIEW.md](docs/LICENSE_REVIEW.md)。

来源与归属详见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)、[vendor/isideload/PROVENANCE.md](vendor/isideload/PROVENANCE.md)：

| 来源 | 贡献 |
|---|---|
| [nab138/iLoader](https://github.com/nab138/iloader) | 原始应用、签名与安装链路 |
| [AndreySemjonov/iloader](https://github.com/AndreySemjonov/iloader) | 本仓库的起始分支：Wi-Fi/RSD 与自动续签 |
| [oacevedo/xtool](https://github.com/oacevedo/xtool/tree/feat/experimental-watchos) | 手表开发者模式准备流程（本仓库为 Rust 原生重写，不含 Python） |
| Data_C0re_ (Rzbck) | Watch Companion 的签名、注册与安装 |
| idevice / isideload 及上游作者 | 设备通信与签名实现 |

## 免责声明

本项目与 Apple Inc. 无任何关联。Apple、iPhone、Apple Watch 是 Apple Inc. 的商标。

仅供学习、研究与个人开发使用；使用者需自行确认其所在地区的法律与所安装应用的许可条款。作者不对设备损坏、数据丢失、账号风控或证书吊销承担责任。
