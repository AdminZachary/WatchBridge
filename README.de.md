<div align="center">

<img src="public/watchbridge.svg" width="104" alt="WatchBridge" />

# WatchBridge

**Entwicklermodus der Apple Watch unter Windows vorbereiten und Watch-Companion-Apps auf die Uhr installieren.**

[English](README.en.md) · [简体中文](README.md) · [한국어](README.ko.md) · [繁體中文](README.zh-TW.md) · [日本語](README.ja.md) · [Español](README.es.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Português](README.pt.md)

[![Build Windows](https://img.shields.io/github/actions/workflow/status/AdminZachary/iloader-for-windows/windows.yml?branch=main&label=windows&logo=github)](https://github.com/AdminZachary/iloader-for-windows/actions/workflows/windows.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows%2010%20%2F%2011-0078D4)
![Status](https://img.shields.io/badge/hardware%20validated-2026--11--12-brightgreen)
![Version](https://img.shields.io/badge/version-0.1.0--alpha.5-lightgrey)

</div>

> [!IMPORTANT]
> WatchBridge ist eine unabhängige Community-Ableitung des [iLoader-Forks von AndreySemjonov](https://github.com/AndreySemjonov/iloader); das Ursprungsprojekt ist [nab138/iLoader](https://github.com/nab138/iloader).
> Es ist **keine offizielle Version von Apple oder eines Upstream-Projekts und impliziert keine Billigung**. Melde Probleme in diesem Repository, nicht upstream.

> [!NOTE]
> Der komplette Ablauf wurde auf echten Geräten verifiziert:
> Entwicklermodus vorbereiten → signieren → auf dem Telefon installieren → auf der Uhr installieren → App starten.

## Was ist das

Um eine selbstsignierte iOS-App (etwa per SideStore) unter Windows zu installieren, braucht es drei Dinge: eine Apple-ID-Signatur, die Registrierung des Zielgeräts als Entwicklungsgerät und den aktivierten Entwicklermodus auf dem Gerät. Die offizielle Toolchain ist Xcode, und die gibt es nur für macOS.

iLoader hat die eine Hälfte (iPhone-Seite) in Rust unter Windows neu gebaut. **WatchBridge ergänzt die andere Hälfte**: Es nimmt das `Watch/*.app` aus der auf dem iPhone installierten App und bringt es auf die gekoppelte Apple Watch; und davor lässt es den Entwicklermodus-Schalter der Uhr erscheinen — ohne Python, ohne Kommandozeile.

Der Weg zum Gerät ist indirekt: Windows sieht die Uhr nicht, deshalb benutzt das Programm das iPhone als Router, leitet einen Port an die Uhr weiter und erledigt Kopplung, Statusabfrage und Installation in dieser weitergeleiteten Sitzung.

## Funktionen

| | Funktion | Details |
|---|---|---|
| 🛠 | **Entwicklermodus vorbereiten** | Uhr koppeln → Schalter anfordern → Status zurücklesen. Es wird nur `action 0` gesendet; **Aktivieren, Neustart und Bestätigen machst immer du auf der Uhr** |
| 📦 | **Companion installieren** | Prüft, ob die IPA sowohl den iPhone-Host als auch `Watch/*.app` enthält, signiert neu, installiert auf dem Telefon und danach die Watch-App |
| 📶 | **Wi-Fi / RSD** | Wi-Fi einmal per USB einrichten, danach drahtlos installieren und verlängern; die Entwicklermodus-Vorbereitung braucht weiterhin USB |
| ♻️ | **Automatische Verlängerung** | Neu signieren vor Ablauf der 7 Tage, mit Tray-Steuerung und optionalem Autostart |
| 🔐 | **Konto und Zertifikate** | Apple-ID-Anmeldung (Geräteidentität über anisette), Zertifikate ansehen/widerrufen, App-ID-Verwaltung, Keyring-Unterstützung |
| 🔗 | **Pairing-Datei** | Pairing-Dateien importieren/exportieren, Wi-Fi-Kopplung reparieren |
| 🌍 | **Mehrsprachige Oberfläche** | 28 Sprachen, darunter Chinesisch (vereinfacht und traditionell) |

## Ablauf

```text
① Gekoppeltes iPhone per USB anschließen   ② Uhr koppeln, Vertrauen auf der Uhr bestätigen
   Gerät auswählen                            Schalter „Entwicklermodus“ anfordern
   ↓                                          ↓
   Auf der Uhr aktivieren → neu starten → bestätigen → „Erneut prüfen“ klicken
   ↓
③ Mit Apple ID anmelden → originale IPA wählen → signieren und installieren
   ↓
   App auf der Uhr selbst öffnen und prüfen, ob sie startet
```

> [!NOTE]
> Das Programm aktiviert den Entwicklermodus **nicht** selbst, startet die Uhr nicht neu und bestätigt kein Vertrauen. Vertrauensdatensätze liegen nur im Speicher; nach einem Neustart des Programms ist eine neue Sitzung nötig.

## Voraussetzungen

- Windows 10 22H2 / Windows 11 (x86_64)
- **Apple Mobile Device Service**: enthalten in [iTunes](https://www.apple.com/itunes/) oder in der Apple-Devices-App aus dem Microsoft Store. Ohne ihn funktionieren weder USB noch Wi-Fi
- WebView2-Runtime (in Windows 11 enthalten)
- Ein iPhone, das bereits mit der Zieluhr gekoppelt ist; die Vorbereitung braucht ein Kabel

## Installation und Start

Lade `watchbridge-windows.zip` von [Releases](https://github.com/AdminZachary/iloader-for-windows/releases/latest), entpacke es und starte `WatchBridge.exe`.

Das Archiv enthält:

| Datei | Inhalt |
|---|---|
| `WatchBridge.exe` | Das Programm, **unsigniert**; SmartScreen warnt beim ersten Start |
| `BUILD.json` | Quell-Commit, ob der Arbeitsbaum Änderungen hatte, Lockfile-Hashes, Toolchain-Versionen, SHA-256 der exe |
| `SHA256SUMS.txt` | Prüfsummen der obigen Dateien |
| `LICENSES.zip` | Lizenztexte Dritter und Abhängigkeitsliste (`inventory.json`) |

## Aus dem Quellcode bauen

Abhängigkeiten und der vollständige Ablauf stehen in [BUILDING.md](BUILDING.md). GitHub Actions kompiliert, führt die Rust-Unit-Tests von `iloader` und `isideload` aus und erzeugt das obige Artefakt.

```powershell
# Nur Frontend
bun install --frozen-lockfile --ignore-scripts
bun run build

# Vollständiger Windows-Build (Tests + exe + Prüfsummen)
.\scripts\Build-Windows.ps1 -BuildExecutable
```

Für einen schnellen lokalen Build ohne Tests `-SkipTests` ergänzen. Der Build muss nativ unter Windows mit MSVC erfolgen; Cross-Compiling wird nicht unterstützt.

## Kompatibilität und bekannte Grenzen

| Grenze | Details |
|---|---|
| **Nur eine Uhr** | Sind am iPhone mehrere Uhren gekoppelt, verweigert die Vorbereitung statt zu raten. Das Weiterleitungsprotokoll hat keinen Parameter „diese Uhr wählen“, und die falsche Uhr zu bedienen ist schlimmer als ein Fehlschlag |
| **Keine Watch-only-Pakete** | Die originale IPA muss iPhone-Host und `Watch/*.app` enthalten |
| **App Group** | Apps, die eine Watch App Group benötigen, können weiterhin vom Upstream-Verhalten betroffen sein, das diese Zuordnung überspringt |
| **Systemversionen** | Der Uhr-Ablauf stammt aus xtools experimenteller Implementierung; jede iOS/watchOS-Kombination braucht Gerätetests |
| **Inoffizielles Werkzeug** | Kein Jailbreak, kein Umgehen der Apple-Signatur, keine gefälschten Zertifikate |

## Häufige Fragen

<details>
<summary><b><code>fail to connect to usbmuxd device socket: io error</code></b></summary>

Unter Windows wird usbmuxd von Apple Mobile Device Service bereitgestellt und lauscht auf `127.0.0.1:27015`. Dieser Fehler bedeutet, dass auf diesem Port niemand antwortet — meist ist der Dienst nicht installiert oder gestoppt.

```powershell
Get-Service "Apple Mobile Device Service"      # sollte existieren und Running sein
Test-NetConnection 127.0.0.1 -Port 27015       # sollte True sein
$env:USBMUXD_SOCKET_ADDRESS                    # sollte leer sein
```

Fehlt der Dienst, installiere iTunes oder die Apple-Devices-App. Wurde die Adresse überschrieben, lösche diese Umgebungsvariable. Das Programm **enthält kein** usbmuxd; es hängt vollständig von Apples Treiber und Dienst ab.
</details>

<details>
<summary><b>„Watch-Informationen können nicht gelesen werden“</b></summary>

Ältere Versionen fragten vor dem Koppeln die Kopplungsregistrierung des iPhones nach dem Uhr-Namen; unter neueren iOS-Versionen liefert diese Abfrage nichts. Die aktuelle Version baut zuerst Vertrauen zur Uhr auf und liest die Identität aus der Sitzung der Uhr selbst. Schlägt es weiterhin fehl, öffne „Einstellungen und Protokolle → Protokolle ansehen“ und prüfe den `kind=`-Wert von `watch step failed`: `timed out` heißt, die Uhr hat nicht geantwortet; `NotFound` / `UnexpectedResponse` deuten auf einen Protokollunterschied hin.
</details>

<details>
<summary><b>„Mehrere gekoppelte Uhren gefunden“</b></summary>

Siehe die Grenze „nur eine Uhr“. Bestätige die Zieluhr in der Watch-App des iPhones; die andere Uhr auszuschalten oder wegzulegen entfernt sie **nicht** aus der Kopplungsregistrierung.
</details>

## Mitwirken

- Regeln: [CONTRIBUTING.md](CONTRIBUTING.md)
- Upstream verbietet ausdrücklich KI-generierte Beiträge; **dieses Repository sendet niemals PRs upstream**. Änderungen bleiben in diesem Fork
- Keine Apple-IDs, Passwörter, privaten Zertifikatsschlüssel, Kopplungsdatensätze oder unredigierte Geräteprotokolle anhängen

## Lizenz und Danksagung

Der Projektcode steht unter MIT ([LICENSE](LICENSE)); Markenbedingungen stehen in [LICENSE-BRANDING](LICENSE-BRANDING).

> Die Signaturbibliothek in der Abhängigkeitskette (`apple-codesign-quick` aus `apple-crates`) deklariert keine Lizenz; die Bedingungen sind weiterhin unklar. Siehe [docs/LICENSE_REVIEW.md](docs/LICENSE_REVIEW.md).

Quellen und Zuordnung: [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md), [vendor/isideload/PROVENANCE.md](vendor/isideload/PROVENANCE.md).

| Quelle | Beitrag |
|---|---|
| [nab138/iLoader](https://github.com/nab138/iloader) | Ursprüngliche App, Signatur- und Installationskette |
| [AndreySemjonov/iloader](https://github.com/AndreySemjonov/iloader) | Ausgangspunkt dieses Repositories: Wi-Fi/RSD und automatische Verlängerung |
| [oacevedo/xtool](https://github.com/oacevedo/xtool/tree/feat/experimental-watchos) | Ablauf zur Entwicklermodus-Vorbereitung (hier nativ in Rust neu geschrieben, ohne Python) |
| Data_C0re_ (Rzbck) | Signatur, Registrierung und Installation des Watch Companion |
| idevice / isideload und ihre Upstream-Autoren | Gerätekommunikation und Signaturimplementierung |

## Haftungsausschluss

Dieses Projekt steht in keiner Verbindung zu Apple Inc. Apple, iPhone und Apple Watch sind Marken von Apple Inc.

Nur für Lernen, Forschung und persönliche Entwicklung; Nutzer müssen die Gesetze ihrer Jurisdiktion und die Lizenzbedingungen der installierten Apps selbst prüfen. Die Autoren haften nicht für Geräteschäden, Datenverlust, Kontosperren oder Zertifikatswiderruf.
