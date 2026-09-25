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
 REST   MCP     CLI     Admin UI
```

Cypht remains responsible for IMAP/JMAP/EWS/SMTP, OAuth, MIME handling and its encrypted user configuration. Gateway provides automation-facing identity, permissions, stable object IDs, idempotency and API contracts.

## v0.4 request flows

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

- `gateway-core` contains stable types, version-2 private object IDs, and the version-1 compatibility decoder.
- `gateway-storage` owns SQLite, migrations and idempotency records.
- `gateway-auth` owns the vault, session tokens, PATs and scopes.
- `gateway-cypht` is the only Rust crate aware of Cypht bridge pages/cookies.
- `gateway-domain` owns user-visible capabilities, validation and policy checks.
- `gateway-api` maps HTTP to domain methods.
- `gateway-mcp` maps MCP tools onto REST v1 and adds AI-context safety wrappers.
- binaries contain startup/configuration only.

REST `/api/v1` is the canonical public contract. CLI/browser UI map to the same domain behavior; MCP intentionally calls REST v1 so remote AI clients cannot bypass PAT scopes, account allow-lists, audit or idempotency.

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

## MCP path

```text
MCP client
  -> stdio token from CYPHT_GATEWAY_TOKEN, or HTTP request bearer PAT
  -> cypht-mcp read/write runtime gate
  -> REST /api/v1
  -> normal Gateway auth/scope/account policy
  -> audit + Cypht bridge
```

Email/attachment output is wrapped as untrusted external content. HTML is omitted by `mail_read`, MCP attachment download is capped at 2 MiB, and upload is capped at 10 MiB.
