# Harden Watch installation, visualize progress and use adaptive transfer

Large Watch companion installs can stall or leave an uncertain outcome after a connection failure. This change integrates seomin0610's [PR #8](https://github.com/AdminZachary/WatchBridge/pull/8), bounds connection setup and cleanup, and prevents automatic resubmission after an uncertain transfer result.

Installation now exposes request-scoped stage progress, transferred bytes and a progress bar. The compact transfer row shows the mode label and live write rate; a translated hint recommends enabling iPhone Wi-Fi. Successful completion still requires the native installation result.

`adaptive_unlimited` is the sole transfer policy, active on normal app launch. It uses the tested V2 windowed write observations and AIMD-style recovery to send without a preset ceiling when the path permits. Fixed and bounded adaptive senders, their configuration variables and unused UI labels have been removed. The V2 controller itself is unchanged.

Validation on macOS arm64:

- TypeScript/Vite production build; 147 application tests and 33 isideload tests passed.
- User-confirmed 200 MiB test installs completed over Wi-Fi (135.03 s) and Bluetooth (838.24 s), with native completion replies. These are Watch-stage durations on different paths, not directly comparable performance benchmarks.
- The user accepted the compact adaptive app after testing. New Wi-Fi and compact rate keys cover all 28 selectable locales; other progress labels use English fallback where untranslated.

Implementation details, the single transfer policy, test scope and limitations are documented in [watch-install-transfer.md](watch-install-transfer.md). Attribution is retained in the vendored source provenance. Test IPAs, application bundles, raw logs and local tooling are excluded.
