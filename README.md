# Cypht Gateway

Cypht Gateway is a Rust-first automation and AI gateway for Cypht. It exposes Cypht's already configured mail accounts through a stable REST API, CLI and management UI while keeping IMAP/JMAP/EWS/SMTP/OAuth handling inside Cypht.

The long-term product also includes MCP. REST v1 is the canonical contract; MCP and other AI surfaces will build on the same Rust domain services rather than reimplementing mail logic.

## Current milestone: v0.2.0

v0.2.0 implements the first real end-to-end Cypht integration:

- Cypht username/password login through Cypht's existing `api_login` module
- private Cypht PHP bridge
- encrypted SQLite gateway credential/session vault
- short-lived access tokens
- persistent PATs with scopes, expiry, revocation and account allow-lists
- signed opaque object IDs
- configured account listing
- mailbox listing
- one-account and unified inbox
- cross-account search
- parsed message reading
- attachment metadata
- browser management UI
- expanded `cyphtctl`
- Git/SemVer, GitHub CI, release automation and multi-architecture image builds

Mail sending/mutation endpoints are intentionally scheduled for v0.3. The `mail.send`, `mail.modify` and `mail.delete` scopes are reserved now so the permission model does not need to be redesigned later.

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
        future MCP
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

The built-in UI can log in to Cypht, create/revoke PATs, list configured accounts, view the unified inbox, search and read messages.

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
cyphtctl inbox
cyphtctl search invoice
cyphtctl read '<opaque-message-id>'
cyphtctl token-create --name agent --scope accounts.read --scope mail.read --scope mail.search
cyphtctl token-revoke tok_...
```

For automated clients, create a PAT in the UI, REST API or CLI and use it as `CYPHT_GATEWAY_TOKEN`. Using `CYPHT_PASSWORD` avoids placing the Cypht password directly in the process command line.

## Docker

`deploy/docker-compose.yml` runs the Rust gateway against an existing/reachable Cypht installation.

```bash
export GATEWAY_MASTER_KEY="$(openssl rand -base64 32)"
export CYPHT_BASE_URL='http://cypht/'
export CYPHT_API_LOGIN_KEY='...'
export CYPHT_BRIDGE_KEY='...'
docker compose -f deploy/docker-compose.yml up --build
```

`docker/Dockerfile.cypht` builds from the pinned Cypht `2.12.2` baseline by default and installs the bridge module. Override `CYPHT_IMAGE` explicitly when testing a newer upstream release.

## Versioning

One SemVer product version is shared by the monorepo. `VERSION` must match `[workspace.package].version` in `Cargo.toml`.

```bash
python3 scripts/check-version.py
python3 scripts/set-version.py 0.3.0
```

Release tags are annotated:

```bash
git tag -a v0.3.0 -m "Cypht Gateway v0.3.0"
```

The API contract is versioned separately through `/api/v1`.

## AI-maintained repository

Read `AGENTS.md` before modifying code. It defines architecture, security invariants, dependency direction and required checks. New capabilities must also update `api/capability-registry.yaml` and `api/openapi.yaml`.

## Repository layout

- `apps/gatewayd` server binary
- `apps/cyphtctl` CLI
- `crates/gateway-core` stable models, errors and opaque IDs
- `crates/gateway-storage` SQLite persistence
- `crates/gateway-auth` vault, sessions, PATs and scopes
- `crates/gateway-cypht` Cypht adapter
- `crates/gateway-domain` business capabilities/policy
- `crates/gateway-api` HTTP/API UI surface
- `cypht-module/gateway` private PHP bridge
- `frontend/static` embedded management UI
- `api` OpenAPI and capability registry
- `docs` architecture/security/compatibility
