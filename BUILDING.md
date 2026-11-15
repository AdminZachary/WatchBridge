# Building on Windows or macOS

Install Python 3.12, Git, Bun, Rust with the Windows MSVC toolchain, and Visual Studio Build
Tools with "Desktop development with C++" and a Windows SDK. The app also needs
the WebView2 runtime. Native dependencies need Perl and a make tool; Strawberry
Perl provides both (CI installs it). Keep the MSVC linker ahead of Unix tools on
PATH: Git's `usr/bin/link.exe` is not the Windows linker.

Run from a fresh checkout in PowerShell:

```powershell
# Install the locked frontend dependencies, build the frontend and run the tests.
.\scripts\Build-Windows.ps1

# Also build the release executable.
.\scripts\Build-Windows.ps1 -BuildExecutable -SkipTests
```

The script runs the TypeScript/Vite build. Without `-SkipTests` it also runs
the Rust unit tests of iloader and the included isideload crate; CI runs them and
records the result in BUILD.json. The executable is built with
`src-tauri/ci.conf.json`, which turns off the original project's updater, and is
copied to `artifacts/windows/WatchBridge.exe` without an installer.
`artifacts/windows/BUILD.json` records the source commit, whether the checkout had
local changes, lockfile hashes, tool versions and the executable's SHA-256;
`SHA256SUMS.txt` lists the hashes. `LICENSES.zip` preserves collected third-party
license texts and a dependency inventory; its pending review status is also
recorded in BUILD.json. See docs/LICENSE_REVIEW.md before public release. Use a clean checkout for release builds.

The script does not run the app or change Windows startup, accounts, pairing or
phone apps. The tests do not use a physical iPhone or Apple Watch.

GitHub Actions runs the same script (`.github/workflows/windows.yml`) and uploads
the executable as a build artifact.

## Building on macOS

The same sources produce a macOS application; the device side uses the system
`usbmuxd` socket (`/var/run/usbmuxd`), so no extra driver is needed.

Requirements: macOS 11 or newer, Xcode command line tools, Rust stable, Bun and
Python 3.12.

```bash
# Checks only: frontend, Rust unit tests, licence inventory.
./scripts/Build-macOS.sh

# Also build WatchBridge.app and package it.
./scripts/Build-macOS.sh --build-executable
```

`--skip-tests` skips the Rust unit tests for a faster build. The helper writes
to `artifacts/macos/`:

| File | Contents |
|---|---|
| `WatchBridge.zip` | `WatchBridge.app`, packaged with `ditto` so the bundle metadata survives |
| `BUILD.json` | Source commit, dirty flag, lockfile hashes, toolchain versions, macOS version, architecture, SHA-256 of the archive |
| `SHA256SUMS.txt` | Checksums of the files above |
| `LICENSES.zip` | Third-party licence texts and the dependency inventory |

`.github/workflows/macos.yml` runs the same script on `macos-14` (arm64) and
`macos-15-intel` (x86_64) and uploads one artifact per architecture.

Three behaviours differ on macOS today:

- The bundle is **ad-hoc signed**, not signed with a Developer ID and not
  notarized. Apple Silicon refuses to execute an unsigned binary, which is why
  the build signs it; but Gatekeeper still treats a downloaded copy as
  untrusted. The first launch is blocked, so approve it once under
  **System Settings → Privacy & Security → Open Anyway**. From a terminal the
  same result is `xattr -dr com.apple.quarantine /Applications/WatchBridge.app`.
  Distributing to other people without that prompt requires a Developer ID
  certificate plus notarization; that is not part of this build.
- Automatic renewal's "start at login" option is Windows-only; on macOS the
  control reports that it is unavailable. Renewal itself works while the app runs.
- The device side uses the system `usbmuxd` socket, so no driver install is
  needed and no Apple Mobile Device Service equivalent must be running.

## Building on Linux

Uses the system `usbmuxd` daemon (socket `/var/run/usbmuxd`); install and start it
(`usbmuxd` package, normally udev-activated when an iPhone is plugged in).

Requirements: Rust stable, Bun, Python 3.12 and the Tauri system libraries.
Debian/Ubuntu: `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
libssl-dev libdbus-1-dev libsecret-1-dev rpm`. Fedora: `webkit2gtk4.1-devel
libappindicator-gtk3-devel librsvg2-devel openssl-devel dbus-devel libsecret-devel rpm-build`.

```bash
./scripts/Build-Linux.sh                      # checks only
./scripts/Build-Linux.sh --build-executable   # also build .deb and .rpm
```

Output goes to `artifacts/linux/` (`.deb`, `.rpm`, `BUILD.json`, `SHA256SUMS.txt`,
`LICENSES.zip`). `.github/workflows/linux.yml` runs it on `ubuntu-22.04` (x86_64).
"Start at login" is Windows-only; it reports unavailable on Linux.

## Dependencies

`vendor/isideload/isideload` is a modified copy of the isideload crate and is
used directly as a path dependency; see `vendor/isideload/PROVENANCE.md`. Other
dependencies come from the registries recorded in `src-tauri/Cargo.lock` and
`bun.lock`, so the first build needs network access. The lockfiles are enforced,
not regenerated.

## Cached and offline builds

You can reuse a prepared `node_modules` directory whose adjacent `bun.lock`
matches this checkout (prepare it with `bun install --frozen-lockfile --ignore-scripts`).

```powershell
.\scripts\Build-Windows.ps1 -BuildExecutable -Offline `
    -FrontendDependencies 'D:\build-cache\iloader\node_modules'
```

`-Offline` stops Cargo from using the network; Rust crate caches must already be
populated. `CARGO_TARGET_DIR` can point to a shared build cache.
