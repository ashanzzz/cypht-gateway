# Cypht Gateway

Cypht Gateway is a Rust-first automation and AI gateway for Cypht. It exposes Cypht's already configured mail accounts through a stable REST API, CLI and management UI while keeping IMAP/JMAP/EWS/SMTP/OAuth handling inside Cypht.

REST v1 is the canonical contract. v0.4 adds an MCP server that calls the same REST policy surface, so AI clients inherit PAT scopes, account allow-lists, idempotency and audit behavior.

## Current milestone: v0.4.0

v0.4.0 turns the mail gateway into a complete AI-facing mail automation surface:

- everything from v0.3 mail read/write, PAT, account allow-lists and opaque IDs
- MCP server based on the official Rust MCP SDK, with stdio and Streamable HTTP transports
- read-only MCP mode by default; write tools require explicit `--allow-write`
- HTTP MCP callers can supply their own Cypht Gateway PAT per request
- AI-safe message/attachment wrappers that label email content as untrusted external data
- attachment context limits for MCP
- audit log REST/CLI/MCP/UI visibility through the new `audit.read` scope
- correlated `X-Request-Id` error responses
- stable idempotency keys required by CLI and reused by the browser compose lifecycle
- `cypht-mcp` included in Docker images and GitHub release artifacts
- registry checks that fail if declared MCP tools are missing from source

Cypht-wide feature parity beyond mail remains incomplete. v0.5 now has local Saved Searches CRUD across REST, CLI, MCP, and the management UI. Live Cypht 2.12.0 Bridge testing remains unverified. Calendar now has a limited local Cypht 2.12.0 list/create/delete surface. Sieve now has a redacted local status surface. Sieve writes, Feeds, settings, and optional PGP remain deferred. Calendar update and unsupported fields remain deferred.

## v0.5 work in progress

Saved Search ID and permission semantics are documented in `docs/SAVED_SEARCHES.md`. Local Contacts, Tags, and Saved Searches have Bridge, REST, CLI, MCP, and UI code. Saved Searches preserve stable owner-bound IDs across Gateway renames, retire IDs after delete, and deny account-restricted PATs. Advanced source references use opaque account and mailbox IDs. A custom Cypht file-settings adapter implements guarded save, stale-writer rejection, and fresh-load readback. Local source tests pass, but live Unraid writes remain unverified. Database-backed user settings still fail closed. Do not use these writes against real data before live tests. Read-only live integration remains unverified. A read-only Docker listing on 2026-09-23 confirmed the Unraid Cypht 2.12.0 container at `http://192.168.8.11:8088/`; no separate Gateway container appeared in that listing. Earlier local-browser observations do not establish the Unraid account state. Other v0.5 capability groups remain pending.

## Architecture

```text
Gmail / QQ / Outlook / JMAP / EWS
                |
              Cypht
                |
        private PHP bridge
                |
         cypht-gateway (Rust)
          |       |       |
        REST     CLI    Admin UI
          |
       cypht-mcp
       /       \
    stdio   Streamable HTTP
```

Public callers never use the PHP bridge directly.

## Cypht prerequisites

The Cypht installation must enable its existing `api_login` module and this repository's `gateway` module.

Copy:

```text
cypht-module/gateway
```

to:

```text
<CYPHT_ROOT>/modules/gateway
```

Append `api_login,gateway` to your existing `CYPHT_MODULES` list. Do not remove the normal Cypht mail modules.

Configure two **different** random secrets in Cypht:

```env
API_LOGIN_KEY=<random secret>
GATEWAY_BRIDGE_KEY=<different random secret>
```

The matching gateway variables are:

```env
CYPHT_API_LOGIN_KEY=<same as API_LOGIN_KEY>
CYPHT_BRIDGE_KEY=<same as GATEWAY_BRIDGE_KEY>
```

## Gateway configuration

Required environment variables (also documented in `.env.example`):

```env
GATEWAY_MASTER_KEY=<base64 32-byte key>
CYPHT_BASE_URL=http://cypht/
CYPHT_API_LOGIN_KEY=...
CYPHT_BRIDGE_KEY=...
```

Generate the gateway master key with:

```bash
openssl rand -base64 32
```

Optional:

```env
GATEWAY_BIND=0.0.0.0:8080
GATEWAY_DB_PATH=/var/lib/cypht-gateway/gateway.db
GATEWAY_SESSION_TTL_SECONDS=3600
# Cypht-side outgoing upload limit: GATEWAY_MAX_UPLOAD_BYTES=20971520
RUST_LOG=gatewayd=info,tower_http=info
```

See `docs/SECURITY.md` before production deployment.

## Run

```bash
cargo run -p gatewayd
```

Open:

```text
http://127.0.0.1:8080/
```

The built-in UI can also compose, upload/download attachments, save drafts, schedule sends, reply/forward, change message state, move/archive/delete messages, inspect audit entries, and show MCP connection examples when the current token has the required scopes.

## REST examples

Login:

```bash
curl -sS http://127.0.0.1:8080/api/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"username":"alice","password":"..."}'
```

Then use the returned access token:

```bash
curl -sS http://127.0.0.1:8080/api/v1/accounts \
  -H "authorization: Bearer $TOKEN"

curl -sS 'http://127.0.0.1:8080/api/v1/messages?limit=50' \
  -H "authorization: Bearer $TOKEN"
```

The complete contract is in `api/openapi.yaml`.

## CLI

```bash
CYPHT_PASSWORD='...' cyphtctl login --username alice

export CYPHT_GATEWAY_TOKEN='cypht_at_...'
cyphtctl accounts
cyphtctl profiles
cyphtctl inbox
cyphtctl search invoice
cyphtctl read '<opaque-message-id>'
cyphtctl upload ./report.pdf --content-type application/pdf
cyphtctl send --to user@example.com --subject hello --body 'hello' --idempotency-key send-20260922-001
cyphtctl reply '<opaque-message-id>' --body 'thanks' --idempotency-key reply-20260922-001
cyphtctl archive '<opaque-message-id>'
cyphtctl token-create --name agent --scope accounts.read --scope mail.read --scope mail.search
cyphtctl token-revoke tok_...
cyphtctl audit --limit 50
cyphtctl saved-searches
cyphtctl saved-search-read '<opaque-saved-search-id>'
cyphtctl saved-search-create --name invoices --kind simple --query invoice
cyphtctl saved-search-create --name advanced-invoices --kind advanced --advanced-file ./advanced-search.json
cyphtctl saved-search-update '<opaque-saved-search-id>' --name invoices-renamed
cyphtctl saved-search-delete '<opaque-saved-search-id>' --yes
cyphtctl calendars
cyphtctl calendar-events <opaque-calendar-id> --start 2026-09-01T00:00:00Z --end 2026-10-01T00:00:00Z
cyphtctl calendar-create <opaque-calendar-id> --title meeting --starts-at 2026-09-24T09:00:00-07:00
cyphtctl calendar-delete <opaque-calendar-id> <opaque-event-id> --yes
```

For automated clients, create a PAT in the UI, REST API or CLI and use it as `CYPHT_GATEWAY_TOKEN`. Using `CYPHT_PASSWORD` avoids placing the Cypht password directly in the process command line.

## MCP

Local stdio mode uses a PAT from the environment:

```bash
export CYPHT_GATEWAY_URL=http://127.0.0.1:8080
export CYPHT_GATEWAY_TOKEN=cypht_pat_...
cypht-mcp --transport stdio
```

Remote Streamable HTTP defaults to loopback and read-only mode:

```bash
cypht-mcp --transport http --bind 127.0.0.1:8790
```

Endpoint: `http://127.0.0.1:8790/mcp`. HTTP clients should send their own `Authorization: Bearer <PAT>` header. By default, `tools/list` hides all write tools and calls to them fail. Use `--allow-write` only when intentional. PAT scopes and account allow-lists still apply. For non-loopback/reverse-proxy deployments, explicitly configure `--allowed-host` and `--allowed-origin`.

## Docker

For local development, copy `deploy/local-dev.env.example` to the ignored `deploy/local-dev.env` file. This profile points Gateway to the existing Cypht upstream at `http://192.168.8.11:8088/`. Use `scripts/run-local.ps1 -HealthOnly` to run local health and UI checks before the Bridge is installed. This mode creates temporary process keys and cannot log in to Cypht. For Bridge tests, fill all three secrets and run `scripts/run-local.ps1` without `-HealthOnly`. The API login and Bridge keys must match the Cypht container.

For the separate local MCP process, point it at the Rust Gateway port:

```bash
cypht-mcp --transport http --bind 127.0.0.1:8790 --url http://127.0.0.1:18080
```

```bash
docker compose --profile edge --env-file deploy/local-dev.env -f deploy/docker-compose.yml up --build
```

This local profile uses a reverse proxy on the current computer at `http://127.0.0.1:8088/`. It routes `/api/v1/` to Gateway, `/mcp` to MCP, and `/` to the Gateway UI. The current Unraid Cypht remains upstream at `http://192.168.8.11:8088/`.

Cypht native UI remains at `http://192.168.8.11:8088/`. Deploy Gateway on a separate, verified-free port, such as `18081`. The edge proxy routes `/` to the Gateway UI, `/api/v1/` to REST, and `/mcp` to MCP. Cypht stays on its private Docker network for Bridge calls.

For Unraid, copy `deploy/unraid.env.example` to the ignored `deploy/unraid.env`, set unique secrets, and confirm host ports before starting Compose. Follow the backup and deployment steps in `docs/UNRAID.md`.



`Dockerfile` builds the Gateway image. `docker/Dockerfile.cypht` builds the private Bridge image on Cypht `2.12.0`. CI builds both images. A release tag publishes versioned images to GHCR. See `docs/RELEASING.md` for version and tag steps.

## Versioning

One SemVer product version is shared by the monorepo. `VERSION` must match `[workspace.package].version` in `Cargo.toml`.

```bash
python3 scripts/set-version.py X.Y.Z
python3 scripts/check-version.py
```

Update all product version files with `scripts/set-version.py`. Create an annotated `vMAJOR.MINOR.PATCH` tag only after all release gates pass. Follow `docs/RELEASING.md`.

The API contract is versioned separately through `/api/v1`.

## AI-maintained repository

Read `AGENTS.md` before modifying code. It defines architecture, security invariants, dependency direction and required checks. New capabilities must also update `api/capability-registry.yaml` and `api/openapi.yaml`.

## Repository layout

- `apps/gatewayd` server binary
- `apps/cyphtctl` CLI
- `apps/cypht-mcp` MCP stdio/HTTP binary
- `crates/gateway-core` stable models, errors and opaque IDs
- `crates/gateway-storage` SQLite persistence
- `crates/gateway-auth` vault, sessions, PATs and scopes
- `crates/gateway-cypht` Cypht adapter
- `crates/gateway-domain` business capabilities/policy
- `crates/gateway-api` HTTP/API UI surface
- `crates/gateway-mcp` AI-safe MCP tools over REST
- `cypht-module/gateway` private PHP bridge
- `frontend/static` embedded management UI
- `api` OpenAPI and capability registry
- `docs` architecture/security/compatibility


Capability completeness is tracked in `docs/PARITY.md`.
