# Code provenance and this fork's changes

This document describes contributions; it does not replace any license.

## Preserved upstream work

- nab138 and iLoader contributors: original desktop application and isideload
  authentication, provisioning, signing and installation architecture.
- AndreySemjonov: the immediate Windows fork, Watch/Wi-Fi/RSD integration,
  installation diagnostics, renewal work and reproducible Windows build scripts.
- Data_C0re_ (Rzbck): Watch companion provisioning, signing and installation
  contributions recorded in `vendor/isideload/PROVENANCE.md`.
- oacevedo / xtool contributors, including original author Kabir Oberai:
  experimental Watch preparation workflow used as the protocol/workflow reference.
- idevice, apple-crates and other dependency authors retain their own copyrights.

All original licenses and notices remain applicable to their own material.
The history of the fork is retained. Copied material is not claimed as original
work by AdminZachary or by an AI provider.

## WatchBridge changes for AdminZachary — 2026-11-12

- A simplified Chinese-first desktop interface with two main tasks.
- Native Rust Watch preparation over USB: explicit trust, developer-mode status,
  AMFI action 0, strict response handling and forwarded-port cleanup.
- Companion IPA validation before signing; the checked source remains retained
  throughout installation. Phone-only packages are rejected.
- A separate app name, icon, data identifier, credential-store service and Windows
  startup registry entry, avoiding collisions with the upstream app.
- Public client-identity fetching from the selected anisette server, with local
  HTTP regression tests. The default third-party service for new installations was explicitly authorized
  by the user; existing preferences are preserved. Actual Apple ID sign-in still
  needs account testing.
- Attribution, validation boundaries, source/version records and license inventory
  included with Windows build output.

These changes were implemented with AI assistance (Codex). Automated compilation,
unit tests and browser tests are recorded separately from physical testing.
This branch has not been validated on an actual iPhone / Apple Watch pair.

## Upstream contribution policy

The original `CONTRIBUTING.md` is preserved. It restricts AI-generated upstream
contributions. This fork's development is at the user's request; no PR or issue
has been submitted to the original project. Any future upstream submission must
follow that project's current contribution requirements and authorship disclosure.
