# WatchBridge hardware acceptance

Record source SHA and binary SHA-256, Windows version, iOS/watchOS versions,
iPhone/Watch models, IPA SHA-256, connection type, and success/failure stage.
Do not publish account identifiers, device UDIDs, signing keys or pairing records.

## Developer mode (physical test still required)

- No device / Network selected: all preparation actions are unavailable.
- USB phone without a Watch: a clear error; no state change.
- Multiple paired Watches: refuse ambiguous selection.
- Check before Pair: discover the Watch; do not create trust automatically.
- Pair: Watch trust prompt appears, deny produces an actionable error, allow succeeds.
- Reveal: AMFI action 0 only. Confirm the setting becomes visible on the Watch.
- Manually enable on the Watch, restart and confirm; Check returns true.
- Disconnect, lock the device, deny trust, and interrupt a forwarded connection:
  errors cannot leave a stale green status or a permanently busy UI.
- Reopen the app: pairing cache is empty. Never infer enabled from discovery alone.

## Installation (physical test still required)

- Missing Apple ID/device/IPA disables installation.
- A phone-only IPA is rejected before signing.
- Select an original companion IPA; keep the exact package through install.
- iPhone install, Watch install, and Watch launch are recorded separately.
- Watch failure after phone success must produce failure, not a completion banner.
- Check both USB and configured Network/RSD transports on the target OS versions.
- No simultaneous preparation, selection change or installation is admitted.
- Test certificate exhaustion and login/2FA failure; never revoke certificates silently.
- App Group / HealthKit capabilities need their own acceptance checks. The existing
  Watch App Group omission is a known limitation, not a validated fix.

## Automated checks

Windows CI performs a locked frontend/production executable build and runs the
Rust unit tests of iloader and isideload. Locally, `-SkipTests` skips them for a
faster build-only run.
Unit tests cannot establish real-device trust, transport compatibility or launch.

## Authentication follow-up

The client identity/user agent are fetched from the selected anisette server;
no hard-coded identity is substituted after errors. Automated tests use local
synthetic HTTP responses only. Real Apple ID, trusted-device 2FA and signing
still need user-account and hardware acceptance.
