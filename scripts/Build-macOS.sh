#!/usr/bin/env bash
# Build the macOS app the same way Build-Windows.ps1 builds the Windows one:
# locked dependencies, optional unit tests, a provenance report and checksums.
set -euo pipefail

build_executable=0
skip_tests=0
while [[ $# -gt 0 ]]; do
    case "$1" in
        --build-executable) build_executable=1 ;;
        --skip-tests) skip_tests=1 ;;
        -h|--help)
            echo "usage: $0 [--build-executable] [--skip-tests]"
            exit 0
            ;;
        *)
            echo "unknown option: $1" >&2
            exit 2
            ;;
    esac
    shift
done

repository="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repository"

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "Use this script on macOS with Xcode command line tools installed." >&2
    exit 1
fi
for program in git bun cargo rustc python3; do
    command -v "$program" >/dev/null 2>&1 || {
        echo "missing required program: $program" >&2
        exit 1
    }
done

commit="$(git rev-parse HEAD)"
if [[ -n "$(git status --porcelain)" ]]; then
    dirty=true
else
    dirty=false
fi

artifacts="$repository/artifacts/macos"
mkdir -p "$artifacts"
# Clear only this helper's own output, so a failed run cannot leave an older
# artifact looking like the result of this one.
rm -f "$artifacts/WatchBridge.zip" "$artifacts/BUILD.json" \
      "$artifacts/SHA256SUMS.txt" "$artifacts/LICENSES.zip"

bun install --frozen-lockfile --ignore-scripts
bun run build

if [[ $skip_tests -eq 0 ]]; then
    for package in iloader isideload; do
        cargo test --locked --manifest-path src-tauri/Cargo.toml --lib -p "$package"
    done
fi

python3 scripts/Collect-Licenses.py --output "$artifacts/LICENSES.zip"

app_zip=""
if [[ $build_executable -eq 1 ]]; then
    bun run tauri build --bundles app --config src-tauri/ci.conf.json -- --locked
    app_path="$repository/src-tauri/target/release/bundle/macos/WatchBridge.app"
    if [[ ! -d "$app_path" ]]; then
        echo "expected app bundle at $app_path" >&2
        exit 1
    fi
    # ditto keeps bundle metadata and the executable bit; plain zip does not.
    ditto -c -k --sequesterRsrc --keepParent "$app_path" "$artifacts/WatchBridge.zip"
    app_zip="$artifacts/WatchBridge.zip"
fi

sha256() {
    shasum -a 256 "$1" | awk '{print $1}'
}

commit_date="$(git show -s --format=%cI HEAD)"
rust_version="$(rustc --version)"
bun_version="$(bun --version)"
macos_version="$(sw_vers -productVersion)"
arch="$(uname -m)"

python3 - "$artifacts" "$commit" "$dirty" "$commit_date" "$rust_version" \
    "$bun_version" "$macos_version" "$arch" "$app_zip" <<'PY'
import hashlib, json, pathlib, sys

artifacts, commit, dirty, commit_date, rust_version, bun_version, macos_version, arch, app_zip = sys.argv[1:]
root = pathlib.Path(artifacts)

def digest(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()

report = {
    "completedUtc": __import__("datetime").datetime.now(__import__("datetime").timezone.utc).isoformat(),
    "sourceCommit": commit,
    "sourceCommitDate": commit_date,
    "sourceDirty": dirty == "true",
    "cargoLockSha256": digest(pathlib.Path("src-tauri/Cargo.lock")),
    "bunLockSha256": digest(pathlib.Path("bun.lock")),
    "rustVersion": rust_version,
    "bunVersion": bun_version,
    "macosVersion": macos_version,
    "architecture": arch,
    "frontend": "passed",
    "physicallyValidated": False,
    "licenseReview": "pending-signing-dependency-clarification; see LICENSES.zip",
    "licensesSha256": digest(root / "LICENSES.zip"),
    "appArchive": "WatchBridge.zip" if app_zip else None,
    "appArchiveSha256": digest(root / "WatchBridge.zip") if app_zip else None,
}
(root / "BUILD.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")

lines = [f'{digest(root / "BUILD.json")}  BUILD.json',
         f'{digest(root / "LICENSES.zip")}  LICENSES.zip']
if app_zip:
    lines.append(f'{digest(root / "WatchBridge.zip")}  WatchBridge.zip')
(root / "SHA256SUMS.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")
PY

echo "Build checks passed. Report: $artifacts/BUILD.json"
if [[ -n "$app_zip" ]]; then
    echo "App archive: $app_zip"
fi
