#!/usr/bin/env python3
"""Create or update a Scoop manifest for vampiro.

Usage: update-scoop.py <manifest_path> <version> <tag> <checksums_path>

Reads the sha256sum checksums file produced by the release workflow and writes
a Scoop bucket manifest for the Windows amd64 build.

The vampiro release ships a single `vampiro.exe` in:
  vampiro_<version>_windows_amd64.zip
"""
import json
import os
import sys

manifest_path = sys.argv[1]
version = sys.argv[2]
tag = sys.argv[3]
checksums_path = sys.argv[4]

sha_win = None
with open(checksums_path) as f:
    for line in f:
        parts = line.split()
        if len(parts) == 2 and "windows_amd64" in parts[1]:
            sha_win = parts[0]
            break

if sha_win is None:
    print("ERROR: windows_amd64 checksum not found", file=sys.stderr)
    sys.exit(1)

url = f"https://github.com/charly-vibes/vampiro/releases/download/{tag}/vampiro_{version}_windows_amd64.zip"

manifest = {
    "version": version,
    "description": "Program analysis tool for verifying compliance with laws and policies",
    "homepage": "https://github.com/charly-vibes/vampiro",
    "license": "Apache-2.0",
    "url": url,
    "hash": f"sha256:{sha_win}",
    "bin": "vampiro.exe",
    "checkver": {
        "github": "https://github.com/charly-vibes/vampiro"
    },
    "autoupdate": {
        "url": "https://github.com/charly-vibes/vampiro/releases/download/v$version/vampiro_$version_windows_amd64.zip"
    }
}

os.makedirs(os.path.dirname(manifest_path), exist_ok=True)
with open(manifest_path, "w") as f:
    json.dump(manifest, f, indent=4)
    f.write("\n")

print(f"Wrote {manifest_path} (version {version})")
print(f"  url: {url}")
print(f"  sha256: {sha_win[:16]}...")
