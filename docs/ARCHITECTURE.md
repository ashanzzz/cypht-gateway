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
 REST   future   CLI     Admin UI
         MCP
```

Cypht remains responsible for IMAP/JMAP/EWS/SMTP, OAuth, MIME handling and its encrypted user configuration. Gateway provides automation-facing identity, permissions, stable object IDs, idempotency and API contracts.

## v0.3 request flows

Read path:

```text
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

Write path:

```text
POST /api/v1/messages/send
  -> bearer + mail.send
  -> Idempotency-Key lookup
  -> opaque profile/upload validation
  -> private PHP bridge
  -> Cypht Hm_Profiles / Hm_MIME_Msg / Hm_SMTP_List
  -> provider SMTP
  -> Cypht Sent mailbox
  -> stored idempotent result
```

Message mutations use the same policy layer before Cypht `Hm_Mailbox` performs read/unread, flag, move, archive or delete actions.

## Dependency rules

- `gateway-core` contains stable types and signed object IDs.
- `gateway-storage` owns SQLite, migrations and idempotency records.
- `gateway-auth` owns the vault, session tokens, PATs and scopes.
- `gateway-cypht` is the only Rust crate aware of Cypht bridge pages/cookies.
- `gateway-domain` owns user-visible capabilities, validation and policy checks.
- `gateway-api` maps HTTP to domain methods.
- binaries contain startup/configuration only.

REST `/api/v1` is the canonical public contract. CLI, browser UI and future MCP must use the same domain behavior rather than reimplementing provider operations.

## Public identifiers

The API does not publish raw Cypht account IDs, folder strings, IMAP UIDs, profile IDs or upload IDs as durable identifiers. It signs typed payloads using the master-key-derived signing key.

Outgoing upload IDs are additionally bound to the gateway username. A PAT cannot reuse another user's temporary upload identifier.

## Idempotent external writes

Send, draft, reply and forward require `Idempotency-Key`. SQLite stores user + operation + key + request hash + response for 24 hours. Repeating the same request returns the original result. Reusing a key for a different request is rejected before contacting Cypht.

## Version sources

- Product version: `VERSION` and Cargo workspace package version.
- Bridge version: `cypht-module/gateway/VERSION`, required to match the Rust gateway exactly.
- API contract: `/api/v1`.
- Release identity: annotated Git tag `v<product-version>`.
- Build identity: Git SHA plus dirty state.
