#!/usr/bin/env bash
# Build the Linux app the same way Build-Windows.ps1 builds the Windows one:
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

if [[ "$(uname -s)" != "Linux" ]]; then
    echo "Use this script on Linux with the Tauri system libraries installed (see BUILDING.md)." >&2
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

artifacts="$repository/artifacts/linux"
mkdir -p "$artifacts"
# Clear only this helper's own output, so a failed run cannot leave an older
# artifact looking like the result of this one.
rm -f "$artifacts"/WatchBridge*.deb "$artifacts"/WatchBridge*.rpm \
      "$artifacts/BUILD.json" "$artifacts/SHA256SUMS.txt" "$artifacts/LICENSES.zip"

version="$(python3 -c 'import json,sys; print(json.load(open("src-tauri/tauri.conf.json"))["version"])')"
arch="$(uname -m)"

bun install --frozen-lockfile --ignore-scripts
bun run build

if [[ $skip_tests -eq 0 ]]; then
    for package in iloader isideload; do
        cargo test --locked --manifest-path src-tauri/Cargo.toml --lib -p "$package"
    done
fi

python3 scripts/Collect-Licenses.py --output "$artifacts/LICENSES.zip"

bundles=""
if [[ $build_executable -eq 1 ]]; then
    bun run tauri build --bundles deb,rpm --config src-tauri/ci.conf.json -- --locked
    bundle_dir="$repository/src-tauri/target/release/bundle"
    cp "$bundle_dir"/deb/*.deb "$bundle_dir"/rpm/*.rpm "$artifacts/"
    bundles="$(cd "$artifacts" && ls WatchBridge*.deb WatchBridge*.rpm | paste -sd, -)"
fi

sha256() {
    sha256sum "$1" | awk '{print $1}'
}

commit_date="$(git show -s --format=%cI HEAD)"
rust_version="$(rustc --version)"
bun_version="$(bun --version)"
linux_version="$(. /etc/os-release && echo "$PRETTY_NAME")"

python3 - "$artifacts" "$commit" "$dirty" "$commit_date" "$rust_version" \
    "$bun_version" "$linux_version" "$arch" "$bundles" <<'PY'
import datetime, hashlib, json, pathlib, sys

artifacts, commit, dirty, commit_date, rust_version, bun_version, linux_version, arch, bundles = sys.argv[1:]
root = pathlib.Path(artifacts)
packages = [b for b in bundles.split(",") if b]

def digest(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()

report = {
    "completedUtc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    "sourceCommit": commit,
    "sourceCommitDate": commit_date,
    "sourceDirty": dirty == "true",
    "cargoLockSha256": digest(pathlib.Path("src-tauri/Cargo.lock")),
    "bunLockSha256": digest(pathlib.Path("bun.lock")),
    "rustVersion": rust_version,
    "bunVersion": bun_version,
    "linuxVersion": linux_version,
    "architecture": arch,
    "frontend": "passed",
    "physicallyValidated": False,
    "licenseReview": "pending-signing-dependency-clarification; see LICENSES.zip",
    "licensesSha256": digest(root / "LICENSES.zip"),
    "packages": {name: digest(root / name) for name in packages},
}
(root / "BUILD.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")

names = ["BUILD.json", "LICENSES.zip", *packages]
(root / "SHA256SUMS.txt").write_text(
    "".join(f"{digest(root / n)}  {n}\n" for n in names), encoding="utf-8")
PY

echo "Build checks passed. Report: $artifacts/BUILD.json"
[[ -n "$bundles" ]] && echo "Packages: $bundles"
