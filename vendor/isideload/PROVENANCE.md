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

## WatchBridge follow-up (2026-11-14)

Apple rejects sign-ins with GrandSlam error -23782 ("A Mac with macOS version
10.13 or later is required") when the `X-Mme-Client-Info` identity sent with
the request claims an OS older than it accepts; public anisette servers
currently report `Mac OS X;10.12.6`. The upstream `set_client_info_override`
mechanism from `nab138/isideload` is ported to
`anisette/remote_v3/mod.rs`: with no override set, the provider still fetches
the identity from its selected server (unchanged behavior); when the caller
sets an override, that identity is used instead. WatchBridge's interactive
sign-in (`src-tauri/src/account.rs`) and saved-session renewal
(`src-tauri/src/renewal/live/saved_session.rs`) apply the same modern Mac
identity that upstream iLoader ships, to be dropped once servers report an
accepted identity again. A synthetic test covers override precedence without
network access.

Update (2026-11-14): the identity uses the same macOS 27.0 version claim as
upstream, but with the shipping RTM build `26A428` instead of upstream's
beta-suffixed `26A5378j`.
