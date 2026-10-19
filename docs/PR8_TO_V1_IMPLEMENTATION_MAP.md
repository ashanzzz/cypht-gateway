# PR8 → v1 implementation map

This map is the proposed slicing after base `96361680ffadf9f0f77e3be8cf88debbb7b67214`. It is intentionally narrower than a "mega PR" so transaction, security and compatibility boundaries can be reviewed independently.

## PR 8.2 — PHP Bridge cohesive account bundle mutation

Active overlay in this bundle.

Files:

- `cypht-module/gateway/account_handlers.php`
- `modules/gateway/account_handlers.php`
- `cypht-module/gateway/user_config.php`
- `modules/gateway/user_config.php`
- `cypht-module/gateway/functions.php`
- `modules/gateway/functions.php`
- `cypht-module/gateway/setup.php`
- `modules/gateway/setup.php`
- `cypht-module/gateway/modules.php`
- `modules/gateway/modules.php`
- `cypht-module/gateway/handler_modules.php`
- `modules/gateway/handler_modules.php`
- `tests/bridge_accounts.php`
- `tests/cypht_account_bundle_persistence.php`
- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`

Completion gate: exactly one durable user-config replacement for each create/update/delete, fresh readback, shared dependency rejection and no secret in any response.

## PR 8.3 — Secret references + account adapter contract

Promote the reference modules only after compile review:

- `future-reference/crates/gateway-auth/src/account_secrets.rs`
- `future-reference/crates/gateway-core/src/account_api.rs`
- `future-reference/crates/gateway-cypht/src/accounts.rs`

Required wiring:

- export `account_secrets` from `gateway-auth`;
- add granular constants `accounts.read`, `accounts.test`, `accounts.write`, `accounts.credentials.write`, `accounts.delete`;
- export request models from `gateway-core`;
- add `mod accounts` in `gateway-cypht`;
- add producer/consumer invariant checks for `accounts.test.v1` and `accounts.write.v1`.

Security gate: `credential_ref` is opaque, owner-bound, encrypted in memory, expires after 300 seconds, is consumed once, has no GET endpoint and never enters SQLite.

## PR 8.4 — Rust account domain lifecycle

Add `crates/gateway-domain/src/accounts.rs` with these boundaries:

1. scope check;
2. request validation;
3. per-user quota;
4. DNS resolution + network policy;
5. consume SecretVault reference or direct REST/CLI secret;
6. ensure/re-authenticate Cypht session;
7. claim operation journal / idempotency;
8. optional candidate test;
9. one PHP bundle mutation call;
10. fresh durable Bridge readback already verified by PHP;
11. persist stable binding + managed bundle;
12. complete operation journal;
13. consume/zeroize temporary secret;
14. metadata-only audit;
15. return protocol-neutral account.

No Rust sequence of three remote IMAP/SMTP/Profile mutations is allowed.

## PR 8.5 — REST v1 account lifecycle

Routes:

- `POST /api/v1/account-secrets`
- `DELETE /api/v1/account-secrets/{id}`
- `GET /api/v1/accounts`
- `GET /api/v1/accounts/{id}`
- `POST /api/v1/accounts`
- `PATCH /api/v1/accounts/{id}`
- `DELETE /api/v1/accounts/{id}`
- `POST /api/v1/accounts/{id}/test`

Public responses never contain Cypht IMAP/SMTP/Profile IDs or credentials.

`GET` returns cached health and never probes provider networks.

## PR 8.6 — MCP + CLI/UI

MCP tools:

- `mail_list_accounts`
- `mail_get_account`
- `mail_add_account`
- `mail_update_account`
- `mail_delete_account`
- `mail_test_account`

MCP schemas contain `credential_ref`, never `password`.

CLI may accept direct credentials only through non-echo input. CLI/UI call REST; they do not bypass policy or call the PHP Bridge.

## PR 9.1 — network policy + discovery

Promote and test `future-reference/crates/gateway-domain/src/network_policy.rs`.

Network invariant:

- resolve every hostname before candidate test/mutation;
- inspect every resolved IP;
- deny loopback/link-local/multicast/unspecified/RFC1918/ULA/CGNAT by default;
- optional admin-only CIDR allowlist;
- no caller-controlled `allow_private` flag;
- repeat validation on redirects or address changes where applicable;
- production egress firewall remains the final boundary.

Discovery:

- curated provider presets;
- DNS SRV when available;
- conservative conventional fallback only;
- discovery returns candidates and never persists an account.

## PR 9.2 — durable operation journal

Promote `future-reference/crates/gateway-storage/src/operation_journal.rs` with a versioned SQLite migration.

States:

`claimed -> in_progress -> succeeded | failed | uncertain`

A timeout/lost Bridge response after mutation marks `uncertain`. The system does not replay a create/update/delete automatically when the remote outcome is unknown.

## PR 9.3 — account health and background work

Rust owns health lifecycle:

- cached health record per public account;
- explicit manual test;
- bounded background refresh jobs;
- no provider probe in a GET request;
- per-account timeout/budget;
- failure metadata is bounded and contains no server banner or credential material.

## PR 9.4 — policy intersection

Enforce:

`Admin Policy ∩ User Policy ∩ IntegrationConnection Policy ∩ Token Scope ∩ Account Allowlist`

MCP annotations remain client metadata and never substitute for authorization.

## PR 10.1 — full contract invariants

Extend `scripts/check-repository.py` to mechanically prove:

- Catalog ↔ OpenAPI exact path + HTTP method;
- Catalog scopes ↔ auth scope constants;
- per-operation OpenAPI scope ↔ Catalog scope;
- Catalog MCP tool set ↔ actual tool set bidirectionally;
- risk ↔ write-tool gating;
- annotation preset ↔ wire annotation values;
- backend capability ↔ Rust consumer constants;
- account write operations require durable backend capability;
- PHP mirrored module trees are byte-identical;
- MCP account write schemas do not contain `password` / `secret` plaintext fields.

## PR 10.2 — release validation and production closure

Required evidence for the final exact SHA:

- repository checker success;
- PHP syntax success;
- Cypht 2.13 primary account behavior tests;
- Cypht 2.12 smoke;
- `cargo fmt --check`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- `cargo test --workspace`;
- release build;
- CI workflow `completed/success`;
- immutable gateway/bridge/AIO full-SHA images;
- current-main preflight and coordinated edge promotion;
- a new annotated tag exercising the redesigned Release workflow;
- release `image-bundle.json` records the authoritative exact image identities.

Cross-repository mutable tag promotion remains coordinated, not atomic.

## v1 stability gate

Do not declare v1 stable until the public Account REST schema, MCP names, CLI command semantics, PAT scope model and compatibility policy have passed a final backwards-compatibility review.
