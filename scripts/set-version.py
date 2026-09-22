#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SEMVER = re.compile(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$")

if len(sys.argv) != 2 or not SEMVER.fullmatch(sys.argv[1]):
    raise SystemExit("usage: scripts/set-version.py MAJOR.MINOR.PATCH[-PRERELEASE][+BUILD]")

version = sys.argv[1]
(ROOT / "VERSION").write_text(version + "\n", encoding="utf-8")

cargo_path = ROOT / "Cargo.toml"
text = cargo_path.read_text(encoding="utf-8")
pattern = re.compile(r"(\[workspace\.package\]\s*\nversion\s*=\s*)\"[^\"]+\"", re.MULTILINE)
text, count = pattern.subn(rf'\1"{version}"', text, count=1)
if count != 1:
    raise SystemExit("could not update [workspace.package].version in Cargo.toml")
cargo_path.write_text(text, encoding="utf-8")

print(f"set product version to {version}")
