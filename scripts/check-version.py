#!/usr/bin/env python3
from __future__ import annotations

import os
import re
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SEMVER = re.compile(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$")


def fail(message: str) -> None:
    print(f"version check failed: {message}", file=sys.stderr)
    raise SystemExit(1)


version = (ROOT / "VERSION").read_text(encoding="utf-8").strip()
if not SEMVER.fullmatch(version):
    fail(f"VERSION is not SemVer: {version!r}")

with (ROOT / "Cargo.toml").open("rb") as fh:
    cargo = tomllib.load(fh)

cargo_version = cargo["workspace"]["package"]["version"]
if cargo_version != version:
    fail(f"Cargo workspace version {cargo_version!r} != VERSION {version!r}")

expected_tag = f"v{version}"
ci_tag = os.environ.get("GITHUB_REF_NAME") if os.environ.get("GITHUB_REF_TYPE") == "tag" else None
if ci_tag and ci_tag != expected_tag:
    fail(f"GitHub tag {ci_tag!r} != expected {expected_tag!r}")

try:
    exact_tag = subprocess.check_output(
        ["git", "describe", "--tags", "--exact-match"], cwd=ROOT, text=True, stderr=subprocess.DEVNULL
    ).strip()
except (subprocess.CalledProcessError, FileNotFoundError):
    exact_tag = ""

if exact_tag and exact_tag != expected_tag:
    fail(f"current Git tag {exact_tag!r} != expected {expected_tag!r}")

print(f"version ok: {version}")
