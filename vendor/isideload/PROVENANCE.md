# isideload source

This directory contains a modified copy of the MIT-licensed
[isideload](https://github.com/nab138/isideload) crate by nab138, which iloader
uses for Apple ID sign-in, signing and installation. It is built from here as a
path dependency, so no other checkout is needed.

Base: `nab138/isideload` at `f6a4d5dba717d72fc2af63eaba26b27ba44116be`
(the revision the original iloader used before it moved to isideload 0.4).
Later upstream fixes ported here: app ID name normalization, retry on HTTP 429
from Apple's sign-in servers, `Connection: close` for sign-in requests, larger
AFC upload chunks and less frequent upload progress logging.

Changes made in this fork:

- Apple Watch companion signing and installation over RSD (`sideload/watch_install.rs`,
  `sideload/bundle.rs`, `sideload/install.rs`, `sideload/sideloader.rs`,
  `sideload/sign.rs`, `sideload/application.rs`). Watch companion support
  includes contributions by Data_C0re_ (Rzbck).
- Watch device registration (`dev/device_type.rs`, `dev/devices.rs`, `dev/app_ids.rs`).
- Certificate and signing identity policies (`sideload/cert_identity.rs`,
  `sideload/builder.rs`, `sideload/certificate_policy_tests.rs`).
- Non-interactive sign-in with saved credentials for automatic renewal
  (`auth/apple_account/noninteractive.rs`, `auth/apple_account.rs`,
  `auth/grandslam.rs`).
- Anisette endpoint normalization (`anisette/mod.rs`, `anisette/remote_v3`).

The crate source, its tests, manifest, upstream README and LICENSE are kept;
repository workflows and examples are not included. Keep LICENSE and this
attribution when updating the copy.

## WatchBridge follow-up (2026-11-12)

The remote anisette provider fetches public `client_info` and `user_agent` from
its already selected server rather than hard-coding a Mac identity. The GET
sends no account/provisioning data. Local synthetic HTTP tests cover identity
refresh, HTTP errors, malformed data and invalid headers. The user explicitly authorized `ani.jkcoxson.com` as the default for new
installations. Existing saved preferences are not overwritten. This is protocol adaptation, not
copied SideStore code; the original authorship and MIT license remain intact.

## WatchBridge follow-up (2026-11-15)

Apple rejects sign-ins with GrandSlam error -23782 ("A Mac with macOS version
10.13 or later is required") when the client identity claims an OS older than
it accepts; public anisette servers currently report `Mac OS X;10.12.6`, and
the check applies to a subset of accounts. Sign-ins go out with the identity
the anisette server reports; on -23782, GrandSlam retries the request once
with a caller-supplied fallback identity and keeps it for the session
(`auth/grandslam.rs`). The fallback uses upstream iLoader's macOS 27.0 claim
with the shipping RTM build `26A428`, and is to be dropped once servers report
an accepted identity themselves.

## WatchBridge local integration of PR #8 (2026-11-24)

Based on [seomin0610's Watch installation improvements](https://github.com/AdminZachary/WatchBridge/pull/8)
at `8fd51211be5fa9c716b381b7eef3ce3401cdf400`: directory checks for embedded
bundles, 32 KiB transfer chunks, Bluetooth pacing, a Wi-Fi forwarding preference,
bounded connection retries, and installation progress logging.

The local integration adds timeouts to USB and RSD setup/forwarding/cleanup,
typed retry classification, and pacing that includes socket write time. Only
connection setup is retried automatically; a transport failure after submitting
the installation is reported as unconfirmed and is not replayed. Confirmed apps
are skipped if setup for a later companion needs reconnecting. The existing
watchOS coordinator-rejection retry remains in place.

The default transfer rate is 256 KiB/s after a 32 MiB initial burst.
`WATCHBRIDGE_WATCH_KBPS=0` disables pacing;
`WATCHBRIDGE_WATCH_UNPACED_MB=0` disables the initial burst. Both settings are
read once per app transfer, and the effective values are logged. Requesting
Wi-Fi does not confirm which transport iOS actually uses. An installation-result
watchdog expires after three minutes without a change of progress or stage,
rather than imposing a total installation deadline.

The local progress-bar follow-up adds optional structured observers to the USB
and RSD installers. Existing callback APIs remain available. Watch transfer
updates count successfully written app-file bytes and are throttled; installation
completion is still determined by the original protocol result, not UI progress.

The optional adaptive sender uses write-duration backoff and gradual recovery,
with a separate finite token allowance shared across files. This is a sending
budget, not a measurement of iPhone memory or Watch acknowledgements. Fixed
pacing remains the default. `WATCHBRIDGE_WATCH_MODE=adaptive` enables probing;
`WATCHBRIDGE_WATCH_MAX_KBPS` bounds the peak (default 4096 KiB/s), while
`WATCHBRIDGE_WATCH_KBPS` controls allowance replenishment. Adaptive bursts are
capped at 32 MiB and unlimited refill is not permitted. This mode requires
physical-device validation separately from the fixed-rate tests.
