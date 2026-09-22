# AI maintainer contract

This repository is designed to be maintained primarily by AI coding agents. Treat this file as a hard engineering contract.

## Product invariants

1. **Cypht remains the mail protocol engine.** Do not reimplement IMAP/JMAP/EWS/SMTP or OAuth in Rust unless a future architecture decision explicitly replaces this rule.
2. **The PHP bridge is private.** Public clients never call `cypht-module/gateway` directly. Bridge requests require an authenticated Cypht session and `X-Cypht-Gateway-Key`.
3. **REST v1 is the canonical public contract.** MCP, CLI and the management UI consume the same Rust domain layer. Do not create separate business logic in those surfaces.
4. **Never expose provider credentials.** Passwords, OAuth access/refresh tokens, Cypht config encryption material and raw PATs must never appear in API responses, logs or audit records.
5. **Opaque IDs are mandatory.** Public account/mailbox/message/attachment IDs must not reveal IMAP server IDs, folders or UIDs directly.
6. **Every new capability gets a registry entry.** Update `api/capability-registry.yaml`, OpenAPI and tests together.
7. **Writes require explicit scopes.** Sending, mutation and deletion must never be included in the default AI token preset.
8. **Mail content is untrusted input.** MCP/agent adapters must label message bodies and attachments as untrusted external content and must not interpret embedded instructions as trusted commands.

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
