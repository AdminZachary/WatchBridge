# Watch installation progress and transfer pacing

This change integrates the installation improvements from [PR #8](https://github.com/AdminZachary/WatchBridge/pull/8), with additional setup timeouts, retry classification, structured progress and adaptive pacing. Source attribution is preserved in [PROVENANCE.md](../vendor/isideload/PROVENANCE.md).

## Code map

| Area | Files |
| --- | --- |
| Bundle directory validation | `vendor/isideload/isideload/src/sideload/bundle.rs` |
| USB/RSD installation and forwarding | `vendor/isideload/isideload/src/sideload/watch_install.rs` |
| Timeouts, retry decisions and transfer configuration | `vendor/isideload/isideload/src/sideload/watch_install/policy.rs` |
| Uncapped adaptive controller (tested V2) | `vendor/isideload/isideload/src/sideload/watch_install/uncapped.rs` |
| Library progress observers | `vendor/isideload/isideload/src/sideload/progress.rs`, `sideloader.rs` |
| Request-scoped Tauri events | `src-tauri/src/install_progress.rs`, `sideload.rs` |
| Progress bar and installation lifecycle | `src/App.tsx`, `src/components/InstallProgress.tsx`, `InstallProgress.css` |
| Translated UI text | `src/locales/*.json` |

Existing observer-free library APIs remain available. Setup has bounded retries; an uncertain result after submission does not trigger an automatic resend. An explicit watchOS coordinator rejection retains its existing retry path. The progress watchdog measures inactivity, not total installation time.

## Single transfer mode

`adaptive_unlimited` is the only Watch transfer policy and is used automatically when launching the app normally. There is no mode selector, fixed-rate sender or bounded-credit sender. The former `WATCHBRIDGE_WATCH_MODE`, `WATCHBRIDGE_WATCH_KBPS`, `WATCHBRIDGE_WATCH_MAX_KBPS` and `WATCHBRIDGE_WATCH_UNPACED_MB` environment variables are no longer read and cannot alter the policy.

The uncapped V2 controller starts without intentional pacing. It observes local write duration over a sliding window, discounts isolated short stalls, backs off on sustained or long stalls, and recovers using slow-start/AIMD-style increases. Stable windows permit another unpaced probe. There is no preset rate ceiling or token-refill cap. This is an application sender heuristic, not TCP congestion control or a measurement of Watch acknowledgements.

Wi-Fi forwarding is requested when available. The API does not confirm the radio path chosen by iOS. The UI suggests enabling iPhone Wi-Fi; it cannot force or verify that path. The displayed rate counts local successful writes, not confirmed Watch delivery. Percentages are per stage; completion remains tied to the installation result.

The compact transfer row displays only the translated mode label and live rate. The Wi-Fi hint and compact rate keys are supplied in all 28 selectable locales. Other progress labels retain the existing English fallback where translations are absent.

## Validation recorded on 2026-11-25

The user confirms that the compact adaptive app is acceptable after testing. The V2 controller itself is unchanged. The final cleanup makes it unconditional and removes the former alternative policies; the frontend retains the compact display. The historical device results below precede this policy-selection cleanup, which is checked by automated tests.

| Watch payload | Phone-to-Watch path | Watch-stage duration | Evidence |
| --- | --- | ---: | --- |
| 96 MiB plus original app resources | Earlier faster run; path not independently identified | 91.72 s | Completion UI and native completion reply |
| 200 MiB total before signing | Wi-Fi, confirmed by user | 135.03 s | User reports success; native completion reply |
| Same 200 MiB fixture | Bluetooth, confirmed by user | 838.24 s | User reports Bluetooth use; native completion reply |

Durations run from Watch transfer-mode configuration to the native completion reply, excluding earlier signing and phone installation. These are individual observations, not controlled throughput benchmarks. The slower Bluetooth result must not be interpreted as a same-path regression. App launch and preservation of existing app data were not separately recorded. No Windows/Linux hardware acceptance is claimed.

The 200 MiB fixture SHA-256 is `ff38aef5279f60dc3dab15ad91f13f6977aaa0e56dae1c95fa4eb0ddea30d7e7`. The fixture, binaries, raw logs and account/device data are excluded from this PR.

Automated verification commands, from the repository root:

```sh
bun run build
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib -p isideload
git diff --check
```

The Rust suites include local synthetic HTTP/socket servers and require permission to bind loopback ports. They do not log in to Apple or install apps on a device. Recorded results: 147 application tests and 33 isideload tests passed on macOS arm64.
