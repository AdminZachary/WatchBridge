# License and branding review

## Confirmed notices

- `LICENSE`: original iLoader MIT notice, copyright (c) 2025 nab138; retained intact.
- `LICENSE-BRANDING`: original name/logo conditions, copyright (C) 2026 nab138;
  retained intact. It permits compliant branding use and does not require a
  derivative to retain the iLoader product name.
- WatchBridge uses its own product name/icon. README and UI explicitly link to
  upstream and state that this is an independent community derivative.
- `vendor/isideload/LICENSE` and `PROVENANCE.md` remain intact.
- xtool's MIT copyright/permission notice is reproduced in THIRD_PARTY_NOTICES.md.
- Python/pymobiledevice3 is not copied or bundled with this native implementation.

## Dependency-chain caveat found during review

The locked signing dependency is `nab138/apple-crates` at
`5da0b8df` (its `apple-codesign-quick` package is named `apple-codesign`). That fork
points to `Dadoum/apple-crates`. At this revision the signer package's Cargo
manifest does not declare a license, and no signer license text was found in the
checkout. The root `LICENSE-lucide.md` covers icons; it must not be represented as
a general license for the signer source.

The fact that iLoader/isideload have MIT licenses does not establish the license
of this separate dependency. Its permission/license needs clarification before
claiming a complete license review or making a public release of the executable.
No such clarification or author permission has been obtained in this task.
Local compilation and source-development progress are recorded independently.

## Build-time inventory

`Collect-Licenses.py` collects declared package licenses, repository references,
authors, and actual license/notice files from installed dependencies. Missing
license declarations/texts are listed in `inventory.json`; the tool never invents
permission or silently relabels a dependency as MIT. The archive is supporting
review material, not an assertion that every dependency is cleared for release.
