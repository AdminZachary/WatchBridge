<div align="center">

<img src="public/watchbridge.svg" width="104" alt="WatchBridge" />

# WatchBridge

**Windows에서 Apple Watch 개발자 모드를 준비하고, Watch Companion 앱을 워치에 설치합니다.**

[English](README.en.md) · [简体中文](README.md) · [한국어](README.ko.md) · [繁體中文](README.zh-TW.md) · [日本語](README.ja.md) · [Español](README.es.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Português](README.pt.md)

[![Build Windows](https://img.shields.io/github/actions/workflow/status/AdminZachary/WatchBridge/windows.yml?branch=main&label=windows&logo=github)](https://github.com/AdminZachary/WatchBridge/actions/workflows/windows.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11%20%C2%B7%20macOS%2011%2B-0078D4)
![Status](https://img.shields.io/badge/hardware%20validated-2026--11--12-brightgreen)
![Version](https://img.shields.io/badge/version-0.1.0--alpha.5-lightgrey)

</div>

> [!IMPORTANT]
> WatchBridge는 [AndreySemjonov의 iLoader 포크](https://github.com/AndreySemjonov/iloader)에서 갈라져 나온 독립 커뮤니티 파생 버전이며, 원본 프로젝트는 [nab138/iLoader](https://github.com/nab138/iloader)입니다.
> Apple 또는 어떤 상위 프로젝트의 **공식 버전이 아니고, 그 어떤 보증도 의미하지 않습니다**. 문제는 이 저장소에 알려 주세요.

> [!NOTE]
> 실기기에서 전체 흐름을 확인했습니다:
> 개발자 모드 준비 → 서명 → 아이폰 설치 → 워치 설치 → 앱 실행.

## 무엇인가

Windows에서 자체 서명 iOS 앱(SideStore 등)을 설치하려면 세 가지가 필요합니다. Apple ID 서명, 대상 기기를 개발용 기기로 등록, 기기의 개발자 모드 활성화입니다. 공식 도구 체인은 Xcode이며 macOS에만 있습니다. 이 앱은 그 부분을 채워 워치 개발자 모드 활성화와 설치의 전체 과정을 Windows와 macOS에서 수행합니다.

## 기능

| | 기능 | 설명 |
|---|---|---|
| 🛠 | **개발자 모드 준비** | 워치 페어링 → 스위치 표시 요청 → 상태 확인. `action 0`만 보내며, **활성화·재시작·확인은 항상 사용자가 워치에서 직접** 합니다 |
| 📦 | **Companion 설치** | IPA에 아이폰 호스트와 `Watch/*.app`이 모두 있는지 확인하고, 재서명 후 아이폰에 설치한 다음 워치 앱을 설치합니다 |
| 📶 | **Wi-Fi / RSD** | USB로 한 번 Wi-Fi를 설정하면 이후 무선으로 설치·갱신할 수 있습니다. 개발자 모드 준비는 여전히 USB가 필요합니다 |
| ♻️ | **자동 갱신** | 7일 서명 만료 전에 자동 재서명, 트레이 제어와 선택적 자동 시작 지원 |
| 🔐 | **계정과 인증서** | Apple ID 로그인(anisette로 기기 신원 제공), 인증서 조회/폐기, App ID 관리, keyring 지원 |
| 🔗 | **페어링 파일** | pairing file 가져오기/내보내기, Wi-Fi 페어링 복구 |
| 🌍 | **다국어 UI** | 28개 언어, 중국어 간체/번체 포함 |

## 사용 흐름

```text
① 페어링된 아이폰을 USB로 연결      ② 워치 페어링, 워치에서 신뢰 확인
   기기 선택                            「개발자 모드」 스위치 표시 요청
   ↓                                    ↓
   워치에서 직접 켜기 → 재시작 → 확인 → 「다시 확인」 클릭
   ↓
③ Apple ID 로그인 → 원본 IPA 선택 → 서명 및 설치
   ↓
   워치에서 직접 앱을 열어 실행되는지 확인
```

> [!NOTE]
> 이 프로그램은 개발자 모드를 대신 켜지 않고, 워치를 재시작하지 않으며, 신뢰 확인을 대신하지 않습니다. 신뢰 기록은 메모리에만 존재하므로 프로그램을 다시 시작하면 새 세션이 필요합니다.

## 시스템 요구 사항

**Windows**

- Windows 10 22H2 / Windows 11 (x86_64)
- **Apple Mobile Device Service**: [iTunes](https://www.apple.com/itunes/) 또는 Microsoft Store의 Apple Devices 앱에 포함됩니다. 없으면 USB와 Wi-Fi 모두 동작하지 않습니다
- WebView2 런타임(Windows 11에 기본 포함)

**macOS**

- macOS 11 이상(Apple Silicon 네이티브, Intel도 지원)
- 드라이버 설치 불필요: 시스템 `usbmuxd`를 그대로 사용합니다
- Xcode 불필요: DMG는 바로 실행됩니다. 소스에서 빌드할 때만 Xcode Command Line Tools가 필요합니다

**공통**

- 대상 워치와 이미 페어링된 아이폰; 준비 단계에는 케이블이 필요합니다

## 설치와 실행

**Windows**: [Releases](https://github.com/AdminZachary/WatchBridge/releases/latest)에서 `watchbridge-windows.zip`을 내려받아 압축을 풀고 `WatchBridge.exe`를 실행하세요.

**macOS**: 아키텍처에 맞는 DMG(Apple Silicon은 `arm64`, Intel은 `x86_64`)를 내려받아 `WatchBridge.app`을 「응용 프로그램」으로 옮기세요. 첫 실행은 Gatekeeper가 차단하므로 「시스템 설정 → 개인정보 보호 및 보안」에서 한 번 「그래도 열기」를 선택합니다.

Windows 압축 파일 구성:

## 소스에서 빌드

의존성과 전체 절차는 [BUILDING.md](BUILDING.md)를 참고하세요. GitHub Actions가 컴파일하고 `iloader`·`isideload` Rust 단위 테스트를 실행한 뒤 위 아티팩트를 만듭니다.

```powershell
# 프런트엔드만 빌드
bun install --frozen-lockfile --ignore-scripts
bun run build

# 전체 Windows 빌드(테스트 + exe + 체크섬 파일)
.\scripts\Build-Windows.ps1 -BuildExecutable
```

빠른 로컬 빌드는 `-SkipTests`를 추가해 테스트를 건너뛸 수 있습니다. 빌드는 Windows + MSVC 네이티브여야 하며 크로스 컴파일은 지원하지 않습니다.

## 호환성과 알려진 제한

| 제한 | 설명 |
|---|---|
| **워치 한 대** | 아이폰에 여러 워치가 페어링되어 있으면 준비 단계에서 추측하지 않고 거부합니다. 포워딩 프로토콜에 "대상 워치 지정" 파라미터가 없고, 잘못된 워치에 작업하는 것이 실패보다 나쁩니다 |
| **Watch-only 패키지 미지원** | 아이폰 호스트와 `Watch/*.app`을 모두 포함한 원본 IPA가 필요합니다 |
| **App Group** | Watch App Group이 필요한 앱은 상위 프로젝트가 해당 연결을 건너뛰는 동작의 영향을 받을 수 있습니다 |
| **OS 버전** | 워치 쪽 흐름은 xtool의 실험적 구현에서 왔으며, iOS/watchOS 조합마다 실기기 검증이 필요합니다 |
| **비공식 도구** | 탈옥 없음, Apple 서명 우회 없음, 인증서 위조 없음 |

## 자주 묻는 질문

<details>
<summary><b><code>fail to connect to usbmuxd device socket: io error</code></b></summary>

Windows에서 usbmuxd는 Apple Mobile Device Service가 제공하며 `127.0.0.1:27015`에서 수신합니다. 이 오류는 해당 포트에 응답하는 대상이 없다는 뜻입니다. 대개 서비스가 설치되지 않았거나 중지된 상태입니다.

```powershell
Get-Service "Apple Mobile Device Service"      # 존재하고 Running이어야 함
Test-NetConnection 127.0.0.1 -Port 27015       # True여야 함
$env:USBMUXD_SOCKET_ADDRESS                    # 비어 있어야 함
```

서비스가 없으면 iTunes 또는 Apple Devices 앱을 설치하세요. 주소가 재정의되어 있다면 해당 환경 변수를 삭제하세요. 이 프로그램은 usbmuxd를 **포함하지 않으며** Apple의 드라이버와 서비스에 전적으로 의존합니다.
</details>

<details>
<summary><b>「워치 정보를 읽을 수 없습니다」</b></summary>

이전 버전은 페어링 전에 아이폰의 페어링 레지스트리에서 워치 이름을 조회했고, 최신 iOS에서는 이 조회가 값을 반환하지 않습니다. 현재 버전은 먼저 워치와 신뢰를 맺고 워치 자체 세션에서 신원을 읽습니다. 그래도 실패하면 「설정 및 로그 → 로그 보기」에서 `watch step failed`의 `kind=` 값을 확인하세요. `timed out`은 워치가 응답하지 않은 것이고, `NotFound` / `UnexpectedResponse`는 프로토콜 차이입니다.
</details>

<details>
<summary><b>「페어링된 워치가 여러 대입니다」</b></summary>

위의 워치 한 대 제한을 참고하세요. 아이폰의 Watch 앱에서 대상 워치를 확인하세요. 다른 워치의 전원을 끄거나 멀리 두는 것은 페어링 레지스트리에서 **사라지지 않습니다**.
</details>

## 기여

- 기여 규칙: [CONTRIBUTING.md](CONTRIBUTING.md)
- 상위 프로젝트는 AI 생성 기여를 명시적으로 금지하므로 **이 저장소는 상위에 PR을 보내지 않습니다**. 변경은 이 포크에 남겨 주세요
- Apple ID, 비밀번호, 인증서 개인 키, 페어링 기록, 마스킹되지 않은 기기 로그를 첨부하지 마세요

## 라이선스와 감사

프로젝트 코드는 MIT로 배포됩니다([LICENSE](LICENSE)). 브랜드 사용 조건은 [LICENSE-BRANDING](LICENSE-BRANDING)에 있습니다.

> 의존성 체인의 서명 라이브러리(`apple-crates`의 `apple-codesign-quick`)는 라이선스를 선언하지 않았으며, 조건은 아직 확인 중입니다. [docs/LICENSE_REVIEW.md](docs/LICENSE_REVIEW.md)를 참고하세요.

출처와 귀속: [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md), [vendor/isideload/PROVENANCE.md](vendor/isideload/PROVENANCE.md).

| 출처 | 기여 |
|---|---|
| [nab138/iLoader](https://github.com/nab138/iloader) | 원본 애플리케이션, 서명 및 설치 파이프라인 |
| [AndreySemjonov/iloader](https://github.com/AndreySemjonov/iloader) | 이 저장소의 출발점: Wi-Fi/RSD와 자동 갱신 |
| [oacevedo/xtool](https://github.com/oacevedo/xtool/tree/feat/experimental-watchos) | 워치 개발자 모드 준비 흐름(이 저장소는 Rust 네이티브로 재작성, Python 없음) |
| Data_C0re_ (Rzbck) | Watch Companion 서명, 등록 및 설치 |
| idevice / isideload 및 상위 저자 | 기기 통신과 서명 구현 |

## 면책

이 프로젝트는 Apple Inc.와 아무런 관련이 없습니다. Apple, iPhone, Apple Watch는 Apple Inc.의 상표입니다.

학습, 연구, 개인 개발 용도로만 사용하세요. 사용자는 거주 지역의 법률과 설치하는 앱의 라이선스 조건을 스스로 확인해야 합니다. 기기 손상, 데이터 손실, 계정 제재, 인증서 폐기에 대해 작성자는 책임지지 않습니다.
