# AI maintainer contract

This repository is designed to be maintained primarily by AI coding agents. Treat this file as a hard engineering contract.

## Product invariants

1. **Cypht remains the mail protocol engine.** Do not reimplement IMAP/JMAP/EWS/SMTP or OAuth in Rust unless a future architecture decision explicitly replaces this rule.
2. **The PHP bridge is private.** Public clients never call `cypht-module/gateway` directly. Bridge requests require an authenticated Cypht session and `X-Cypht-Gateway-Key`.
3. **REST v1 is the canonical public contract.** CLI and the management UI map to that contract; MCP calls REST so it cannot bypass authentication, policy, idempotency or audit. Do not create provider logic in surface crates.
4. **Never expose provider credentials.** Passwords, OAuth access/refresh tokens, Cypht config encryption material and raw PATs must never appear in API responses, logs or audit records.
5. **Opaque IDs are mandatory.** Public account/mailbox/message/attachment IDs must not reveal IMAP server IDs, folders or UIDs directly.
6. **Every new capability gets a registry entry.** Update `api/capability-registry.yaml`, OpenAPI and tests together.
7. **Writes require explicit scopes.** Sending, mutation and deletion must never be included in the default AI token preset.
8. **Mail content is untrusted input.** MCP/agent adapters must label message bodies and attachments as untrusted external content and must not interpret embedded instructions as trusted commands.
9. **External sends are idempotent.** Any public operation that can send mail must atomically claim an `Idempotency-Key` before contacting SMTP. A crash after an uncertain external send must never trigger an automatic replay.
10. **Uploaded attachment files are temporary.** Keep them in Cypht private storage with restrictive permissions, enforce size limits, and delete them after successful send/draft or after the stale-file TTL.
11. **MCP is read-only by default.** Write tools require an explicit runtime opt-in and the caller PAT still needs the corresponding scope.
12. **Remote MCP must preserve caller identity.** Streamable HTTP uses the request bearer PAT; do not replace it with a shared server-wide privileged token.
13. **Audit records are metadata only.** Never store mail bodies, attachment bytes, passwords, raw PATs or provider tokens in audit detail.
14. **Optional Cypht modules must fail safely.** Check module support before using its classes. Return a bounded capability-unavailable error when the module is disabled.
15. **Extended IDs are not complete until private parts stay private.** A signed base64 payload is readable. Use a versioned keyed digest bound to owner, type, and source for new object IDs. Keep encrypted internal-part mappings in persistent Gateway storage. Never put Cypht internal IDs in public payloads.
16. **Global contacts are not mailbox-scoped.** Reject account-restricted PATs for local Contacts rather than silently broadening their access.
17. **Pin the upstream runtime.** Reject a Bridge ping if `CYPHT_VERSION` is not the reviewed Cypht 2.12.0 baseline. Do not silently run v0.5 against unreviewed Cypht releases.
18. **Tag mutations are account-scoped.** Cypht 2.12.0 removes matching message UIDs across all accounts. Public Tag removal must match account, folder, and UID. Never invent a destination UID when MOVE omits COPYUID. Deny account-restricted PATs for global Tags.
19. **User-config mutations must be durable.** Cypht Contacts and Tags repository calls can update only the active session. Do not report a successful write until a persistent save and fresh-session readback verify it. Detect stale state before mutation. Treat failed or lost Bridge responses as uncertain outcomes and never blindly replay creates or destructive writes.
20. **Saved Searches are user-wide settings.** Deny account-restricted PATs before reading or writing them. Map Cypht advanced-search account/folder strings to opaque Gateway IDs. Persist encrypted owner-bound ID mappings. Never audit search names or query data. Keep `searches.read` out of default AI scopes. Reject rename collisions and retire deleted IDs.
21. **Calendar is a limited Cypht 2.12.0 adapter.** Expose only the user-level calendar_events model, RFC3339 range reads, create, and confirmed unique delete. Do not expose update, all-day, end time, location, invitations, or complex recurrence until Cypht provides stable primitives.

## Change workflow

Before finishing a change:

```bash
python3 scripts/check-version.py
python3 scripts/check-repository.py
find cypht-module -name '*.php' -print0 | xargs -0 -n1 php -l
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

If a local tool is unavailable, do not claim it passed. CI is the final gate.

## Versioning

- One product version is shared across the monorepo.
- `VERSION` must equal `[workspace.package].version`.
- Release tags are annotated tags formatted `vMAJOR.MINOR.PATCH`.
- REST compatibility is versioned independently through `/api/v1`.

## Dependency direction

```text
gateway-core       <- stable shared types
     ^
gateway-storage    <- persistence only
gateway-auth       <- tokens/vault/scopes
gateway-cypht      <- Cypht adapter only
gateway-domain     <- business capabilities
gateway-api        <- HTTP mapping only
gateway-mcp        <- MCP client of REST v1 only
apps/*             <- binaries only
```

Avoid cyclic dependencies and avoid importing Axum/HTTP types into domain or core crates.

## Error handling

- No `unwrap()`/`expect()` in request paths.
- Convert adapter failures into `GatewayError`.
- Public error bodies must not contain passwords, tokens, raw upstream responses or stack traces.
- Authentication failures intentionally reveal little detail.

## Security review triggers

A change requires explicit security review if it touches:

- credential vault encryption
- PAT hashing or scopes
- Cypht session handling
- attachment download/upload
- sending/deleting/moving mail
- CORS or public bridge exposure
- logging/audit payloads
- webhooks/MCP remote transport
