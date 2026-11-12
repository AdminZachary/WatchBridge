# Building on Windows

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
the Rust unit tests of iloader and the included isideload crate. CI uses
`-SkipTests` at the user's request and records that explicitly in BUILD.json. The executable is built with
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
