"""Collect existing notices and dependency provenance without inventing licenses."""
import argparse
import json
from pathlib import Path
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]

def collect(destination):
    # Decode explicitly as UTF-8 with replacement: text=True would use the
    # platform encoding (CP1252 on Windows), and Cargo output can contain bytes
    # that codepage cannot represent, which aborts the build with a
    # UnicodeDecodeError instead of a useful message.
    metadata = json.loads(subprocess.check_output([
        'cargo', 'metadata', '--locked', '--format-version', '1',
        '--manifest-path', str(ROOT / 'src-tauri/Cargo.toml'),
    ], cwd=ROOT).decode('utf-8', errors='replace'))
    entries = []
    destination.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(destination, 'w', zipfile.ZIP_DEFLATED, strict_timestamps=False) as archive:
        for name in ['LICENSE', 'LICENSE-BRANDING', 'THIRD_PARTY_NOTICES.md',
                     'vendor/isideload/LICENSE', 'vendor/isideload/PROVENANCE.md',
                     'docs/CONTRIBUTIONS.md', 'docs/LICENSE_REVIEW.md']:
            archive.write(ROOT / name, 'project/' + name)
        for package in metadata['packages']:
            directory = Path(package['manifest_path']).parent
            files = [p for p in directory.iterdir() if p.is_file() and
                     p.name.upper().startswith(('LICENSE', 'LICENCE', 'COPYING', 'NOTICE', 'COPYRIGHT'))]
            if package.get('license_file'):
                candidate = directory / package['license_file']
                if candidate.is_file() and candidate not in files:
                    files.append(candidate)
            prefix = f"rust/{package['name']}-{package['version']}"
            for index, path in enumerate(sorted(files)):
                archive.write(path, f'{prefix}/{index}-{path.name}')
            entries.append({'ecosystem': 'rust', 'name': package['name'],
                            'version': package['version'], 'license': package.get('license'),
                            'repository': package.get('repository'), 'source': package.get('source'),
                            'authors': package.get('authors', []),
                            'licenseFiles': [p.name for p in files]})
        for manifest in sorted((ROOT / 'node_modules').rglob('package.json')):
            # Only package-root manifests, not examples nested inside a package.
            relative = manifest.relative_to(ROOT / 'node_modules')
            if len(relative.parts) not in (2, 3) or (len(relative.parts) == 3 and not relative.parts[0].startswith('@')):
                continue
            package = json.loads(manifest.read_text(encoding='utf-8-sig'))
            files = [p for p in manifest.parent.iterdir() if p.is_file() and
                     p.name.upper().startswith(('LICENSE', 'LICENCE', 'COPYING', 'NOTICE', 'COPYRIGHT'))]
            name = package.get('name', str(relative.parent))
            version = package.get('version', 'unknown')
            for path in sorted(files):
                archive.write(path, f'npm/{name}/{version}/{path.name}')
            entries.append({'ecosystem': 'npm', 'name': name, 'version': version,
                            'license': package.get('license'), 'repository': package.get('repository'),
                            'author': package.get('author'), 'contributors': package.get('contributors', []),
                            'licenseFiles': [p.name for p in files]})
        report = {'reviewStatus': 'pending-signing-dependency-clarification', 'packages': entries,
                  'missingLicenseDeclarations': [f"{p['ecosystem']}:{p['name']}@{p['version']}" for p in entries if not p['license']],
                  'missingLicenseFiles': [f"{p['ecosystem']}:{p['name']}@{p['version']}" for p in entries if not p['licenseFiles']]}
        archive.writestr('inventory.json', json.dumps(report, ensure_ascii=False, indent=2))
    print(f'Collected {len(entries)} package records. License review remains pending; see docs/LICENSE_REVIEW.md.')

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, required=True)
    collect(parser.parse_args().output)
