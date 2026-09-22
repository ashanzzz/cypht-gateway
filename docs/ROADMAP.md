# Roadmap

## v0.1.x — foundation ✓

- Git/SemVer foundation
- build metadata
- API daemon skeleton
- CLI skeleton
- Docker/GitHub build automation

## v0.2.x — bridge, auth and readable mail ✓

- private Cypht PHP bridge
- Cypht username/password login through `api_login`
- encrypted SQLite credential/session vault
- PAT management, scopes, expiry and revocation
- opaque signed object IDs
- accounts and mailboxes
- unified inbox and search
- parsed message body and attachment metadata
- management UI and expanded CLI

## v0.3.x — complete mail writes ✓

- SMTP/profile discovery
- send/reply/reply-all/forward
- upload/download attachments
- drafts and scheduled send
- flags/read/unread
- move/archive/trash/delete
- idempotency keys for external sends and draft creation
- matching REST, CLI and browser UI operations

## v0.4.x — Cypht feature coverage

- tags and saved searches
- contacts
- calendars/events
- Sieve filters and block list
- feeds
- settings/profile administration
- optional PGP integration

## v0.5.x — AI and automation surfaces

- MCP stdio and Streamable HTTP
- complete AI-safe tool surface
- SSE events
- signed webhooks
- audit UI
- Cypht compatibility matrix

## v1.0.0

Stable REST v1, MCP tool naming, CLI commands, PAT model and documented compatibility policy.
