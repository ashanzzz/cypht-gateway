# Architecture

## Product boundary

```text
Mail providers
     |
     v
   Cypht  <---- normal Cypht web UI
     |
     v
private Cypht PHP bridge
     |
     v
Rust gateway
  |       |       |       |
 REST    MCP     CLI     Admin UI
```

Cypht remains responsible for IMAP/JMAP/EWS/SMTP, OAuth, MIME handling and its encrypted user configuration. Gateway provides automation-facing identity, permissions, stable object IDs and API contracts.

## v0.2 request flow

```text
POST /api/v1/auth/login
  -> gateway-domain
  -> gateway-cypht
  -> Cypht api_login
  -> hm_id + hm_session
  -> encrypted SQLite gateway session

GET /api/v1/messages
  -> bearer authentication
  -> scope/account policy
  -> validate or refresh Cypht session
  -> private PHP bridge
  -> Hm_IMAP_List / Hm_Mailbox
  -> normalized domain models
  -> opaque signed IDs
  -> JSON
```

## Dependency rules

- `gateway-core` contains stable types and signed object IDs.
- `gateway-storage` owns SQLite and migrations.
- `gateway-auth` owns the vault, session tokens, PATs and scopes.
- `gateway-cypht` is the only Rust crate aware of Cypht bridge pages/cookies.
- `gateway-domain` owns user-visible capabilities and policy checks.
- `gateway-api` maps HTTP to domain methods.
- binaries contain startup/configuration only.

## Public identifiers

The API does not publish raw Cypht account IDs, folder strings or IMAP UIDs as durable identifiers. It signs typed payloads using the master-key-derived signing key. A message ID internally represents account + mailbox + provider message UID while remaining opaque to clients.

## Version sources

- Product version: `VERSION` and Cargo workspace package version.
- API contract: `/api/v1`.
- Release identity: annotated Git tag `v<product-version>`.
- Build identity: Git SHA plus dirty state.
