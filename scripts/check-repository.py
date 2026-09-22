#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
errors: list[str] = []

required = [
    ".env.example",
    "AGENTS.md",
    "LICENSE",
    "VERSION",
    "api/openapi.yaml",
    "api/capability-registry.yaml",
    "docs/SECURITY.md",
    "cypht-module/gateway/setup.php",
    "cypht-module/gateway/VERSION",
    "cypht-module/gateway/handler_modules.php",
    "frontend/static/index.html",
]
for name in required:
    if not (ROOT / name).is_file():
        errors.append(f"missing required file: {name}")

root_version = (ROOT / "VERSION").read_text(encoding="utf-8").strip()
bridge_version_file = ROOT / "cypht-module/gateway/VERSION"
if bridge_version_file.is_file():
    bridge_version = bridge_version_file.read_text(encoding="utf-8").strip()
    if bridge_version != root_version:
        errors.append(f"Cypht bridge version {bridge_version!r} != product version {root_version!r}")

cargo = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
members = re.findall(r'^\s*"([^"]+)",?\s*$', cargo, re.M)
for member in members:
    if member.startswith(("crates/", "apps/")) and not (ROOT / member / "Cargo.toml").is_file():
        errors.append(f"workspace member lacks Cargo.toml: {member}")

openapi = (ROOT / "api/openapi.yaml").read_text(encoding="utf-8")
registry = (ROOT / "api/capability-registry.yaml").read_text(encoding="utf-8")
for method, path in re.findall(r'api:\s*"(GET|POST|PATCH|PUT|DELETE) (/api/v1/[^" ]+)"', registry):
    relative = path.removeprefix("/api/v1")
    normalized = re.sub(r"\{[^}]+\}", "{id}", relative)
    openapi_norm = re.sub(r"\{[^}]+\}", "{id}", openapi)
    if normalized not in openapi_norm:
        errors.append(f"capability route missing from OpenAPI: {method} {path}")

# Prevent accidental literal deployment secrets in tracked config/docs.
secret_pattern = re.compile(r"^(GATEWAY_MASTER_KEY|CYPHT_API_LOGIN_KEY|CYPHT_BRIDGE_KEY)=([^<$\s].+)$", re.M)
for path in ROOT.rglob("*"):
    if not path.is_file() or ".git" in path.parts or path.suffix in {".zip", ".png", ".jpg"}:
        continue
    try:
        text = path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        continue
    for match in secret_pattern.finditer(text):
        value = match.group(2).strip()
        if value not in {"...", "<random secret>", "<different random secret>", "<base64 32-byte key>"}:
            errors.append(f"possible hard-coded secret in {path.relative_to(ROOT)}: {match.group(1)}")

if errors:
    for error in errors:
        print(f"repository check failed: {error}", file=sys.stderr)
    raise SystemExit(1)
print("repository check ok")
