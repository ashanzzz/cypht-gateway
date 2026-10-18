#!/usr/bin/env python3
from __future__ import annotations

import yaml

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
    "docs/ACCOUNTS_PLAN.md",
    "docs/SAVED_SEARCHES.md",
    "docs/CALENDAR.md",
    "docs/SIEVE.md",
    "docs/CYPHT_MODULE_MATRIX.md",
    "docs/UNRAID.md",
    "docs/RELEASING.md",
    "docs/VALIDATION.md",
    "deploy/nginx/gateway-edge.conf",
    ".github/workflows/ci.yml",
    ".github/workflows/image.yml",
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
    "tests/bridge_capabilities.php",
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

# Container publication invariants: one immutable build owner, one release alias owner.
image_workflow_path = ROOT / ".github/workflows/image.yml"
release_workflow_path = ROOT / ".github/workflows/release.yml"
if image_workflow_path.is_file() and release_workflow_path.is_file():
    image_workflow = image_workflow_path.read_text(encoding="utf-8")
    release_workflow = release_workflow_path.read_text(encoding="utf-8")
    full_sha_tag = "type=raw,value=sha-${{ env.SOURCE_SHA }}"
    bridge_package = "ghcr.io/${{ github.repository }}-cypht-bridge"

    if "workflow_run:" not in image_workflow or 'workflows: ["CI"]' not in image_workflow:
        errors.append("Container Images must be triggered from the completed CI workflow")
    if "github.event.workflow_run.conclusion == 'success'" not in image_workflow:
        errors.append("Container Images must require a successful CI conclusion")
    if 'tags: ["v*"]' in image_workflow:
        errors.append("Container Images must not publish release-tag aliases")
    if image_workflow.count(full_sha_tag) != 3:
        errors.append("all three image builds must publish full Git-SHA identities")
    if "type=sha,prefix=sha-" in image_workflow:
        errors.append("short SHA image tags are forbidden; use the full Git SHA")
    if bridge_package not in image_workflow or bridge_package not in release_workflow:
        errors.append("Container and Release workflows must use the canonical -cypht-bridge package")
    if "docker/build-push-action@" in release_workflow:
        errors.append("Release must promote existing immutable images, not rebuild containers")
    if "image-bundle.json" not in release_workflow:
        errors.append("Release must publish the authoritative immutable image bundle manifest")
    if "needs.validate.outputs.source_sha" not in release_workflow or "sha-${SOURCE_SHA}" not in release_workflow:
        errors.append("Release must resolve image identities from the annotated tag commit SHA")

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
    errors.append("missing Cypht runtime baseline or Docker image reference")
elif baseline_match.group(1) != image_match.group(1):
    errors.append(
        f"Rust Cypht baseline {baseline_match.group(1)!r} != Docker baseline {image_match.group(1)!r}"
    )

ci_workflow_path = ROOT / ".github/workflows/ci.yml"
if ci_workflow_path.is_file() and release_workflow_path.is_file():
    ci_workflow = ci_workflow_path.read_text(encoding="utf-8")
    release_workflow = release_workflow_path.read_text(encoding="utf-8")
    pin_pattern = re.compile(
        r"repository:\s*cypht-org/cypht\s+ref:\s*([0-9a-f]{40})", re.M
    )
    ci_pin = pin_pattern.search(ci_workflow)
    release_pin = pin_pattern.search(release_workflow)
    if not ci_pin or not release_pin:
        errors.append("CI and Release must pin the primary Cypht source to an exact commit")
    elif ci_pin.group(1) != release_pin.group(1):
        errors.append("CI and Release use different pinned primary Cypht source commits")
    if baseline_match:
        baseline = baseline_match.group(1)
        for workflow_name, workflow in (("CI", ci_workflow), ("Release", release_workflow)):
            if f"Cypht {baseline} primary" not in workflow:
                errors.append(f"{workflow_name} does not identify Cypht {baseline} as the primary source baseline")

# Capability producer vs consumer check
rust_caps = set(
    re.findall(
        r'^pub const CAP_[A-Z0-9_]+: &str = "([^"]+)";',
        bridge_adapter,
        re.M,
    )
)
bridge_php_path = ROOT / "cypht-module/gateway/handler_modules.php"
if bridge_php_path.is_file():
    bridge_php = bridge_php_path.read_text(encoding="utf-8")
    php_cap_literals = set(
        re.findall(r"'([a-z0-9_.]+\.v[0-9]+)'", bridge_php)
    )
    missing_bridge_caps = rust_caps - php_cap_literals
    if missing_bridge_caps:
        errors.append(
            "Rust consumes Bridge capabilities not represented by PHP: "
            + ", ".join(sorted(missing_bridge_caps))
        )

# Bidirectional zero-drift check between cypht-module/gateway and modules/gateway
canonical_dir = ROOT / "cypht-module/gateway"
mirror_dir = ROOT / "modules/gateway"

canonical = {p.name: p for p in canonical_dir.glob("*.php")}
mirror = {p.name: p for p in mirror_dir.glob("*.php")}

missing = canonical.keys() - mirror.keys()
extra = mirror.keys() - canonical.keys()

for name in sorted(missing):
    errors.append(f"missing mirrored Gateway module: modules/gateway/{name}")

for name in sorted(extra):
    errors.append(f"unexpected mirrored Gateway module: modules/gateway/{name}")

for name in sorted(canonical.keys() & mirror.keys()):
    if canonical[name].read_bytes() != mirror[name].read_bytes():
        errors.append(f"Gateway PHP mirror drift: {name}")

cargo = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
members = re.findall(r'^\s*"([^"]+)",?\s*$', cargo, re.M)
for member in members:
    if member.startswith(("crates/", "apps/")) and not (ROOT / member / "Cargo.toml").is_file():
        errors.append(f"workspace member lacks Cargo.toml: {member}")

openapi_text = (ROOT / "api/openapi.yaml").read_text(encoding="utf-8")
registry = (ROOT / "api/capability-registry.yaml").read_text(encoding="utf-8")
openapi_data = yaml.safe_load(openapi_text)
openapi_paths = openapi_data.get("paths", {})

# Exact OpenAPI path and HTTP method validation
for method, path in re.findall(r'api:\s*"(GET|POST|PATCH|PUT|DELETE) (/api/v1/[^" ]+)"', registry):
    relative = path.removeprefix("/api/v1")
    if not relative.startswith("/"):
        relative = "/" + relative
    method_lower = method.lower()
    if relative not in openapi_paths:
        errors.append(f"capability route path missing from OpenAPI: {method} {path}")
    elif method_lower not in openapi_paths[relative]:
        errors.append(f"capability HTTP method {method} for {path} missing from OpenAPI paths")

scope_enum = re.search(
    r"(?ms)^    Scope:\n      type: string\n      enum: \[([^]]+)\]",
    openapi_text,
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

# Exact Gateway API router path and HTTP method validation
api_source_path = ROOT / "crates/gateway-api/src/lib.rs"
if api_source_path.is_file():
    api_source = api_source_path.read_text(encoding="utf-8")
    routes = {}
    for m in re.finditer(r'\.route\(\s*"(/api/v1[^"]*)",\s*([\s\S]*?)\)(?=\s*\.(?:route|layer|fallback|with_state)|;)', api_source):
        r_path = m.group(1)
        chain = m.group(2)
        methods = set()
        if re.search(r"(?:^|\.|\s)get\(", chain):
            methods.add("get")
        if re.search(r"(?:^|\.|\s)post\(", chain):
            methods.add("post")
        if re.search(r"(?:^|\.|\s)(?:axum::routing::)?patch\(", chain):
            methods.add("patch")
        if re.search(r"(?:^|\.|\s)delete\(", chain):
            methods.add("delete")
        routes[r_path] = methods

    for method, path in re.findall(r'api:\s*"(GET|POST|PATCH|PUT|DELETE) (/api/v1/[^" ]+)"', registry):
        method_lower = method.lower()
        if path not in routes:
            errors.append(f"capability route path missing from gateway-api router: {method} {path}")
        elif method_lower not in routes[path]:
            errors.append(f"capability HTTP method {method} for {path} missing from gateway-api router (has {routes[path]})")

# PR6 Operation Catalog and MCP Tool invariants
mcp_source_path = ROOT / "crates/gateway-mcp/src/lib.rs"
if mcp_source_path.is_file():
    mcp_source = mcp_source_path.read_text(encoding="utf-8")
    if "planned_mcp:" in registry:
        errors.append("capability registry still contains planned_mcp entries; v0.4 requires real mcp mappings")

    catalog_data = yaml.safe_load(registry)

    # 1. Bi-directional MCP tool set exact equality
    catalog_mcp_tools = set(re.findall(r"^\s*mcp:\s*([A-Za-z0-9_]+)\s*$", registry, re.M))
    mcp_source_tools = set(re.findall(r"#\[tool\([^)]*?name\s*=\s*\"([^\"]+)\"", mcp_source))
    missing_in_source = catalog_mcp_tools - mcp_source_tools
    missing_in_catalog = mcp_source_tools - catalog_mcp_tools
    if missing_in_source:
        errors.append("MCP tools in catalog but missing in source: " + ", ".join(sorted(missing_in_source)))
    if missing_in_catalog:
        errors.append("MCP tools in source but missing in catalog: " + ", ".join(sorted(missing_in_catalog)))

    # 2. Bi-directional Write tool set exact equality
    write_tools = set()
    for capability in re.split(r"(?m)^  [A-Za-z][A-Za-z0-9_.]*:\s*$", registry)[1:]:
        tool = re.search(r"(?m)^\s+mcp:\s*([A-Za-z0-9_]+)\s*$", capability)
        risk = re.search(r"(?m)^\s+risk:\s*([a-z_]+)\s*$", capability)
        if tool and risk and risk.group(1) in {"write", "destructive", "external_write"}:
            write_tools.add(tool.group(1))
    gated_tools = re.search(r"(?s)const WRITE_TOOLS: &\[&str\] = &\[(.*?)\];", mcp_source)
    registered_gates = set(re.findall(r'"([A-Za-z0-9_]+)"', gated_tools.group(1))) if gated_tools else set()
    if write_tools != registered_gates:
        errors.append("MCP write-tool gate differs from capability registry: " +
                      ", ".join(sorted(write_tools ^ registered_gates)))

    # 3. Tool annotation presets and risk consistency
    valid_presets = set(catalog_data.get("annotation_presets", {}).keys())
    expected_presets = {"read", "write", "update", "destructive", "send"}
    if valid_presets != expected_presets:
        errors.append("catalog annotation presets differ from standard: " + ", ".join(sorted(valid_presets ^ expected_presets)))

    for op_name, op in catalog_data.get("capabilities", {}).items():
        if "mcp" in op:
            tool_info = op.get("tool", {})
            annotation = tool_info.get("annotation") if isinstance(tool_info, dict) else None
            if not annotation or annotation not in valid_presets:
                tool_name = op["mcp"]
                errors.append(f"operation {op_name} ({tool_name}) has invalid or missing tool annotation: {annotation}")
            risk = op.get("risk")
            if risk == "read" and annotation != "read":
                errors.append(f"operation {op_name} has risk read but annotation {annotation}")
            elif risk == "destructive" and annotation != "destructive":
                errors.append(f"operation {op_name} has risk destructive but annotation {annotation}")
            elif risk == "external_write" and annotation != "send":
                errors.append(f"operation {op_name} has risk external_write but annotation {annotation}")
            elif risk == "write" and annotation not in {"write", "update"}:
                errors.append(f"operation {op_name} has risk write but annotation {annotation}")

        # 4. Backend capabilities consistency
        backend_caps = op.get("backend_capabilities", [])
        for cap in backend_caps:
            if cap not in rust_caps:
                errors.append(f"operation {op_name} lists unknown backend capability: {cap}")

# 5. Tool annotation metadata exact match in #[tool(... annotations(...))]
    tool_expected_preset = {}
    for op_name, op in catalog_data.get("capabilities", {}).items():
        if "mcp" in op:
            tool_expected_preset[op["mcp"]] = op.get("tool", {}).get("annotation")

    tool_matches = re.finditer(r"#\[tool\(([\s\S]*?)\)\]", mcp_source)
    verified_tool_annotations = 0
    for tm in tool_matches:
        attr = tm.group(1)
        name_m = re.search(r'name\s*=\s*"([^"]+)"', attr)
        if not name_m:
            continue
        tool_name = name_m.group(1)
        preset_name = tool_expected_preset.get(tool_name)
        if not preset_name:
            continue
        expected_hints = catalog_data["annotation_presets"][preset_name]
        ann_m = re.search(r"annotations\(([\s\S]*?)\)", attr)
        if not ann_m:
            errors.append(f"MCP tool {tool_name} missing annotations(...) metadata in #[tool]")
            continue
        ann_body = ann_m.group(1)
        for hint_key, expected_val in [
            ("read_only_hint", expected_hints["read_only"]),
            ("destructive_hint", expected_hints["destructive"]),
            ("idempotent_hint", expected_hints["idempotent"]),
            ("open_world_hint", expected_hints["open_world"]),
        ]:
            hint_m = re.search(rf"{hint_key}\s*=\s*(true|false)", ann_body)
            if not hint_m:
                errors.append(f"MCP tool {tool_name} missing {hint_key} in annotations(...)")
            elif (hint_m.group(1) == "true") != expected_val:
                actual_val = (hint_m.group(1) == "true")
                errors.append(f"MCP tool {tool_name} {hint_key} mismatch: actual={actual_val}, expected={expected_val} from preset {preset_name}")
        verified_tool_annotations += 1
    if verified_tool_annotations != len(catalog_mcp_tools):
        errors.append(f"expected {len(catalog_mcp_tools)} tool annotations verified, found {verified_tool_annotations}")


# Prevent accidental literal deployment secrets in tracked config/docs.
secret_pattern = re.compile(r"^(GATEWAY_MASTER_KEY|CYPHT_API_LOGIN_KEY|CYPHT_BRIDGE_KEY|UNRAID_USER|UNRAID_PW)=([^<$\s].+)$", re.M)
for path in ROOT.rglob("*"):
    if not path.is_file() or ".git" in path.parts or "target" in path.parts or path.suffix in {".zip", ".png", ".jpg"}:
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
