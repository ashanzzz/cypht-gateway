# Architecture

## Product boundary

Cypht Gateway will provide stable automation interfaces over Cypht without reimplementing Cypht's mail-provider protocol stack.

```text
Mail providers
     |
     v
   Cypht  <---- normal Cypht web UI
     |
     v
Cypht PHP bridge  (private/internal only)
     |
     v
Rust domain/service layer
  |       |       |       |
 REST    MCP     CLI     Admin UI
```

## Rules

1. Public clients never call the PHP bridge directly.
2. REST/OpenAPI is the public machine contract.
3. MCP, CLI, and the management UI reuse the same Rust domain capabilities.
4. Cypht remains responsible for IMAP, JMAP, EWS, SMTP, OAuth, MIME handling, and encrypted Cypht user configuration.
5. Gateway is responsible for API authentication, PATs, scopes, account allow-lists, audit logs, rate limits, webhooks/events, and AI-facing normalization.
6. Public object identifiers are opaque. Cypht/IMAP implementation identifiers must not become permanent API contracts.
7. Write operations such as mail sending will support idempotency keys.

## Version sources

- Product version: `VERSION` and Cargo workspace package version.
- API contract generation: `/api/v1`.
- Release identity: annotated Git tag `v<product-version>`.
- Build identity: Git SHA plus dirty state.
