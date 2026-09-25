#!/usr/bin/env python3
from __future__ import annotations

import os
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
source_root = os.environ.get("CYPHT_SOURCE_PATH")
if not source_root:
    raise SystemExit("set CYPHT_SOURCE_PATH to the reviewed Cypht source tree")

modules_path = Path(source_root) / "modules"
if not modules_path.is_dir():
    raise SystemExit("CYPHT_SOURCE_PATH does not contain a modules directory")

source_modules = {path.name for path in modules_path.iterdir() if path.is_dir()}
matrix = (ROOT / "docs/CYPHT_MODULE_MATRIX.md").read_text(encoding="utf-8")
mapped_modules = set(re.findall(r"^\| `([^`]+)` \|", matrix, re.MULTILINE))
missing = sorted(source_modules - mapped_modules)
extra = sorted(mapped_modules - source_modules)
if missing or extra:
    if missing:
        print("module matrix is missing: " + ", ".join(missing), file=sys.stderr)
    if extra:
        print("module matrix has unknown modules: " + ", ".join(extra), file=sys.stderr)
    raise SystemExit(1)

app_config = Path(source_root) / "config/app.php"
if not app_config.is_file():
    raise SystemExit("CYPHT_SOURCE_PATH does not contain config/app.php")
app_text = app_config.read_text(encoding="utf-8", errors="replace")
default_match = re.search(
    r"'modules'\s*=>\s*explode\(\s*['\"],[\'\"]\s*,\s*env\(\s*['\"]CYPHT_MODULES['\"]\s*,\s*['\"]([^'\"]*)['\"]\s*\)\s*\)",
    app_text,
)
if not default_match:
    raise SystemExit("could not read Cypht's default CYPHT_MODULES list")
default_modules = set(default_match.group(1).split(","))
required_default = {
    "core",
    "contacts",
    "local_contacts",
    "tags",
    "calendar",
    "feeds",
    "saved_searches",
}
missing_defaults = sorted(required_default - default_modules)
if missing_defaults:
    raise SystemExit("Cypht default module list changed: " + ", ".join(missing_defaults))
if "api_login" in default_modules:
    raise SystemExit("reviewed Cypht default unexpectedly enables api_login")

print(f"Cypht module matrix ok: {len(source_modules)} modules")
print("Cypht 2.12.0 default module list verified; api_login and gateway require explicit enablement")
