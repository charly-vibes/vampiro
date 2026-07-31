#!/usr/bin/env python3
"""Create or update a Homebrew formula for vampiro.

Usage: update-homebrew.py <formula_path> <version> <tag> <checksums_path>

Reads the sha256sum checksums file produced by the release workflow and writes
a Homebrew formula covering macOS (arm64/amd64) and Linux (arm64/amd64).

The vampiro release ships a single `vampiro` binary per platform archive:
  vampiro_<version>_<platform>.tar.gz
"""
import os
import sys

formula_path = sys.argv[1]
version = sys.argv[2]
tag = sys.argv[3]
checksums_path = sys.argv[4]

shas = {}
with open(checksums_path) as f:
    for line in f:
        parts = line.split()
        if len(parts) == 2:
            sha, name = parts
            for platform in ["darwin_arm64", "darwin_amd64", "linux_arm64", "linux_amd64"]:
                if platform in name:
                    shas[platform] = sha

base = f"https://github.com/charly-vibes/vampiro/releases/download/{tag}"

formula = f"""\
# typed: false
# frozen_string_literal: true

class Vampiro < Formula
  desc "Program analysis tool for verifying compliance with laws and policies"
  homepage "https://github.com/charly-vibes/vampiro"
  version "{version}"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "{base}/vampiro_{version}_darwin_arm64.tar.gz"
      sha256 "{shas.get('darwin_arm64', '')}"
    end
    on_intel do
      url "{base}/vampiro_{version}_darwin_amd64.tar.gz"
      sha256 "{shas.get('darwin_amd64', '')}"
    end
  end

  on_linux do
    on_arm do
      if Hardware::CPU.is_64_bit?
        url "{base}/vampiro_{version}_linux_arm64.tar.gz"
        sha256 "{shas.get('linux_arm64', '')}"
      end
    end
    on_intel do
      url "{base}/vampiro_{version}_linux_amd64.tar.gz"
      sha256 "{shas.get('linux_amd64', '')}"
    end
  end

  def install
    bin.install "vampiro"
  end

  test do
    system "#{{bin}}/vampiro", "--version"
  end
end
"""

os.makedirs(os.path.dirname(formula_path), exist_ok=True)
with open(formula_path, "w") as f:
    f.write(formula)

print(f"Wrote {formula_path} (version {version})")
for platform in ["darwin_arm64", "darwin_amd64", "linux_arm64", "linux_amd64"]:
    s = shas.get(platform, "")
    print(f"  {platform}: {s[:16]}{'...' if s else '(missing)'}")
