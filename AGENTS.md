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
17. **Dynamic upstream runtime.** Log and audit `CYPHT_VERSION` on Bridge ping while maintaining compatibility across Cypht 2.12.0+ baselines. Do not reject newer upstream releases merely due to string mismatches. Follow upstream Cypht releases dynamically.
18. **Tag mutations are account-scoped.** Cypht 2.12.0 removes matching message UIDs across all accounts. Public Tag removal must match account, folder, and UID. Never invent a destination UID when MOVE omits COPYUID. Deny account-restricted PATs for global Tags.
19. **User-config mutations must be durable.** Cypht Contacts and Tags repository calls can update only the active session. Do not report a successful write until a persistent save and fresh-session readback verify it. Detect stale state before mutation. Treat failed or lost Bridge responses as uncertain outcomes and never blindly replay creates or destructive writes.
20. **Saved Searches are user-wide settings.** Deny account-restricted PATs before reading or writing them. Map Cypht advanced-search account/folder strings to opaque Gateway IDs. Persist encrypted owner-bound ID mappings. Never audit search names or query data. Keep `searches.read` out of default AI scopes. Reject rename collisions and retire deleted IDs.
21. **Calendar is a limited Cypht 2.12.0 adapter.** Expose only the user-level calendar_events model, RFC3339 range reads, create, and confirmed unique delete. Do not expose update, all-day, end time, location, invitations, or complex recurrence until Cypht provides stable primitives.

## Engineering decision discipline

AI agents must optimize for correctness, simplicity, and measured system behavior rather than language preference, architectural fashion, or visual uniformity.

### Language and process boundaries

- Do not move functionality to Rust merely because Rust is newer, statically typed, or perceived as faster.
- Do not keep functionality in PHP merely because it already exists there.
- Put Cypht-native protocol and configuration semantics in the PHP adapter when Cypht already owns the relevant objects, state, transaction boundaries, or protocol behavior.
- Put authentication, authorization policy, OAuth, MCP transport, durable scheduling, idempotency, operation journals, cross-user audit, process isolation, and other independent control-plane state in Rust.
- A language-boundary change requires a concrete reason such as state ownership, lifecycle isolation, concurrency requirements, security isolation, or a measured performance bottleneck.
- Do not claim a performance improvement from language choice alone. Measure representative latency, CPU, allocations, lock contention, and network time first.
- Every additional RPC, serialization boundary, database, queue, cache, crate, abstraction, or service must justify the failure modes and maintenance cost it introduces.

### Preserve semantics before preserving shape

Code uniformity is not an engineering goal by itself.

Before replacing multiple call sites with a shared helper, classify each affected operation by:

- read vs write;
- remote provider mutation vs local Cypht user-config mutation;
- whether provider credentials or a Cypht config key are actually required;
- whether durable local persistence is required;
- whether the operation has an external side effect;
- whether the operation is idempotent;
- whether a lost response can leave an uncertain outcome;
- whether its capability is deployment-, user-, account-, mailbox-, or session-specific.

A refactor MUST NOT change any of those properties unless the change explicitly intends to change product semantics and has tests for the new behavior.

Do not add credential parameters "for consistency". Secrets must cross a boundary only when the downstream operation actually requires them.

Transport primitives must transport requests. Authorization, capability policy, durability policy, and business semantics belong above raw transport and must not be silently embedded into generic HTTP helpers.

### Capability contract rules

Bridge capabilities describe implementation availability, not caller authorization.

Public scopes answer:

> Is this caller allowed to perform the operation?

Bridge capabilities answer:

> Can this Bridge deployment implement the operation contract?

Account or mailbox capabilities answer:

> Can this particular account or mailbox perform the operation?

Do not mix these layers.

`BridgePing` capabilities may contain deployment/backend properties only because Bridge metadata can be cached across sessions. Never put user-, account-, mailbox-, credential-, or session-specific facts in the global Bridge capability cache.

For every capability consumed by Rust, verify all of the following before claiming the capability is implemented:

1. the Rust consumer names the capability;
2. the PHP Bridge can advertise the exact capability string;
3. optional-module capabilities are advertised only when that module is actually available;
4. a test covers both capability-present and capability-missing behavior;
5. the capability is represented in the operation registry when it belongs to a public operation.

A capability existing only on the consumer side is a broken contract.

A capability existing only on the producer side is allowed for forward compatibility.

### Evidence before completion claims

Never use implementation intent as evidence.

After modifying code, inspect the final repository state again. Do not rely on the patch you intended to write, an earlier diff, or your own change summary.

Use these status terms precisely:

- `design-only`: described but not implemented;
- `implemented`: code exists;
- `locally-tested`: relevant local checks passed;
- `CI-green`: the required workflow completed successfully;
- `validated`: representative integration/runtime behavior was exercised;
- `closed`: every explicitly defined completion gate is satisfied.

These states are not interchangeable:

`implemented != tested != CI-green != validated != closed`

Never say "fully fixed", "100% covered", "complete", "all methods", "no remaining bypass", or equivalent language unless that property is mechanically verified against the final source tree.

If a custom local script is used as evidence for a recurring invariant, either:

- commit that check to the repository and run it in CI; or
- describe the result only as a local observation, not as a repository guarantee.

CI success proves only the invariants CI actually tests. A green CI run does not prove semantic properties that have no corresponding test or checker.

### Semantic diff review

Before finishing any refactor, explicitly inspect whether the change altered:

- authorization or required scopes;
- capability requirements;
- secrets crossing process boundaries;
- persistence requirements;
- network request count;
- timeout behavior;
- transaction boundaries;
- external side effects;
- idempotency behavior;
- retry behavior;
- uncertain-outcome handling;
- public IDs or information exposure.

Unexpected changes in any of these dimensions are regressions even when formatting, Clippy, unit tests, and compilation pass.

### Repeated AI mistakes become executable invariants

When the same class of mistake is discovered more than once, do not solve it only by adding another prose reminder.

Prefer, in order:

1. make the invalid state unrepresentable in types or interfaces;
2. add a focused unit/integration test;
3. add a small repository invariant checker;
4. only then add or clarify prose in this file.

Examples include:

- Rust consuming a Bridge capability that PHP never advertises;
- mirrored PHP Bridge sources drifting apart;
- a public adapter method bypassing its required capability gate;
- a secret being passed to an operation that does not require it;
- a mutable image tag being published before all release artifacts succeed.

### Release truthfulness

Never call a workflow green while its status is not `completed` with conclusion `success`.

Do not infer that a release is coherent merely because every image eventually built successfully.

Immutable commit-SHA image identities are authoritative.

Mutable tags such as `edge`, semantic-version aliases, and `latest` must only be promoted after all required artifacts for the same commit have succeeded.

`latest` is release-only and must never be moved by an ordinary `main` branch push.

### Completion review

Before declaring a change closed:

1. re-read every final file materially changed by the work;
2. search for old call paths or bypasses the change was intended to eliminate;
3. compare producer and consumer contracts across process boundaries;
4. run the repository checks and relevant focused tests;
5. confirm required GitHub Actions runs reached `completed/success`;
6. state any property that remains untested or only locally verified.

The purpose of the final review is to try to disprove the agent's own completion claim.

## Change workflow

Before finishing a change:

```bash
python3 scripts/check-version.py
python3 scripts/check-repository.py
python3 scripts/check-frontend.py
find cypht-module -name '*.php' -print0 | xargs -0 -n1 php -l
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release --workspace
```

If a local tool is unavailable, do not claim it passed. CI is a required gate, not proof of untested semantic properties. Do not report CI as green until the workflow itself is completed with conclusion=success. Successful individual jobs are not sufficient.

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
