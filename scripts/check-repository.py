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
    "docs/PARITY.md",
    "docs/SAVED_SEARCHES.md",
    "docs/CALENDAR.md",
    "docs/SIEVE.md",
    "docs/CYPHT_MODULE_MATRIX.md",
    "docs/UNRAID.md",
    "docs/RELEASING.md",
    "docs/VALIDATION.md",
    "deploy/nginx/gateway-edge.conf",
    ".github/workflows/ci.yml",
    ".github/workflows/release.yml",
    "scripts/check-cypht-matrix.py",
    "scripts/run-local.ps1",
    "cypht-module/gateway/setup.php",
    "cypht-module/gateway/VERSION",
    "cypht-module/gateway/handler_modules.php",
    "cypht-module/gateway/contacts.php",
    "cypht-module/gateway/contact_handlers.php",
    "cypht-module/gateway/tags.php",
    "cypht-module/gateway/tag_handlers.php",
    "cypht-module/gateway/saved_search_handlers.php",
    "cypht-module/gateway/calendar_handlers.php",
    "cypht-module/gateway/sieve_handlers.php",
    "docker/cypht-gateway-config.php",
    "tests/bridge_contract.php",
    "tests/bridge_ping.php",
    "tests/cypht_api_login_config.php",
    "tests/cypht_user_config_persistence.php",
    "tests/cypht_contacts_repo.php",
    "tests/cypht_saved_searches_repo.php",
    "tests/bridge_saved_searches.php",
    "tests/bridge_calendar.php",
    "tests/bridge_sieve_status.php",
    "tests/cypht_tags_repo.php",
    "tests/bridge_tags.php",
    "frontend/static/index.html",
    "crates/gateway-mcp/src/lib.rs",
    "apps/cypht-mcp/src/main.rs",
]
for name in required:
    if not (ROOT / name).is_file():
        errors.append(f"missing required file: {name}")

edge_path = ROOT / "deploy/nginx/gateway-edge.conf"
if edge_path.is_file():
    edge = edge_path.read_text(encoding="utf-8")
    for marker in (
        "listen 8088;",
        "location ^~ /api/v1/",
        "location = /mcp",
        "proxy_pass http://$gateway_backend;",
        "proxy_pass http://$mcp_backend;",
        "return 404;",
        "ajax_gateway_",
    ):
        if marker not in edge:
            errors.append(f"edge proxy is missing required private/API routing: {marker}")

compose_path = ROOT / "deploy/docker-compose.yml"
if compose_path.is_file():
    compose = compose_path.read_text(encoding="utf-8")
    if 'profiles: ["mcp", "edge"]' not in compose:
        errors.append("MCP must be enabled by the optional edge profile")
    if 'GATEWAY_MCP_ALLOW_WRITE: "false"' in compose:
        errors.append("unexpected MCP write env name")
    if 'CYPHT_MCP_ALLOW_WRITE: "false"' not in compose:
        errors.append("MCP write mode must be off by default in Compose")
    if 'GATEWAY_EDGE_HOST_PORT:-8088' not in compose:
        errors.append("edge profile must publish the configured host port with 8088 default")

root_version = (ROOT / "VERSION").read_text(encoding="utf-8").strip()
bridge_version_file = ROOT / "cypht-module/gateway/VERSION"
if bridge_version_file.is_file():
    bridge_version = bridge_version_file.read_text(encoding="utf-8").strip()
    if bridge_version != root_version:
        errors.append(f"Cypht bridge version {bridge_version!r} != product version {root_version!r}")

bridge_adapter = (ROOT / "crates/gateway-cypht/src/lib.rs").read_text(encoding="utf-8")
cypht_image = (ROOT / "docker/Dockerfile.cypht").read_text(encoding="utf-8")
cypht_runtime_config_path = ROOT / "docker/cypht-gateway-config.php"
cypht_runtime_config = cypht_runtime_config_path.read_text(encoding="utf-8") if cypht_runtime_config_path.is_file() else ""
if "COPY docker/cypht-gateway-config.php /usr/local/share/cypht/config/gateway.php" not in cypht_image:
    errors.append("Cypht image must install the runtime API login config")
if "api_login_key" not in cypht_runtime_config or "API_LOGIN_KEY" not in cypht_runtime_config:
    errors.append("Cypht runtime config must source API_LOGIN_KEY without embedding a secret")
baseline_match = re.search(r'^const CYPHT_BASELINE_VERSION: &str = "([^"]+)";', bridge_adapter, re.M)
image_match = re.search(r'^ARG CYPHT_IMAGE=cypht/cypht:([^\s]+)$', cypht_image, re.M)
if not baseline_match or not image_match:
    errors.append("missing Cypht runtime baseline or pinned Docker image")
elif baseline_match.group(1) != image_match.group(1):
    errors.append(f"Cypht runtime baseline {baseline_match.group(1)} != Docker image {image_match.group(1)}")
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



scope_enum = re.search(
    r"(?ms)^    Scope:\n      type: string\n      enum: \[([^]]+)\]",
    openapi,
)
if not scope_enum:
    errors.append("OpenAPI Scope enum is missing")
else:
    openapi_scopes = set(re.findall(r"[A-Za-z][A-Za-z0-9_.]*", scope_enum.group(1)))
    auth_source = (ROOT / "crates/gateway-auth/src/lib.rs").read_text(encoding="utf-8")
    auth_scopes = set(re.findall(r'^pub const SCOPE_\w+: &str = "([^"]+)";', auth_source, re.M))
    registry_scopes = set(re.findall(r"^\s*scope: ([A-Za-z][A-Za-z0-9_.]*)$", registry, re.M))
    for additional in re.findall(r"additional_scopes: \[([^]]+)\]", registry):
        registry_scopes.update(re.findall(r"[A-Za-z][A-Za-z0-9_.]*", additional))
    missing_scopes = (auth_scopes | registry_scopes) - openapi_scopes
    if missing_scopes:
        errors.append("scope values missing from OpenAPI: " + ", ".join(sorted(missing_scopes)))

api_source_path = ROOT / "crates/gateway-api/src/lib.rs"
if api_source_path.is_file():
    api_source = api_source_path.read_text(encoding="utf-8")
    for method, path in re.findall(r'api:\s*"(GET|POST|PATCH|PUT|DELETE) (/api/v1/[^" ]+)"', registry):
        route_path = path.removeprefix("/api/v1")
        if f'"/api/v1{route_path}"' not in api_source:
            errors.append(f"capability route missing from gateway-api source: {method} {path}")

# Every MCP tool declared in the capability registry must exist in the MCP source.
mcp_source_path = ROOT / "crates/gateway-mcp/src/lib.rs"
if mcp_source_path.is_file():
    mcp_source = mcp_source_path.read_text(encoding="utf-8")
    if "planned_mcp:" in registry:
        errors.append("capability registry still contains planned_mcp entries; v0.4 requires real mcp mappings")
    for tool in re.findall(r"^\s*mcp:\s*([A-Za-z0-9_]+)\s*$", registry, re.M):
        if f'name = "{tool}"' not in mcp_source:
            errors.append(f"registry MCP tool missing from source: {tool}")
    write_tools = set()
    for capability in re.split(r"(?m)^  [A-Za-z][A-Za-z0-9_.]*:\s*$", registry)[1:]:
        tool = re.search(r"(?m)^    mcp:\s*([A-Za-z0-9_]+)\s*$", capability)
        risk = re.search(r"(?m)^    risk:\s*([a-z_]+)\s*$", capability)
        if tool and risk and risk.group(1) in {"write", "destructive", "external_write"}:
            write_tools.add(tool.group(1))
    gated_tools = re.search(r"(?s)const WRITE_TOOLS: &\[&str\] = &\[(.*?)\];", mcp_source)
    registered_gates = set(re.findall(r'"([A-Za-z0-9_]+)"', gated_tools.group(1))) if gated_tools else set()
    if write_tools != registered_gates:
        errors.append("MCP write-tool gate differs from capability registry: " +
                      ", ".join(sorted(write_tools ^ registered_gates)))

# Prevent accidental literal deployment secrets in tracked config/docs.
secret_pattern = re.compile(r"^(GATEWAY_MASTER_KEY|CYPHT_API_LOGIN_KEY|CYPHT_BRIDGE_KEY|UNRAID_USER|UNRAID_PW)=([^<$\s].+)$", re.M)
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
