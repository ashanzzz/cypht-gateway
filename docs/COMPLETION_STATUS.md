# Cypht Gateway completion bundle status

Base repository: `ashanzzz/cypht-gateway`

Exact base SHA: `96361680ffadf9f0f77e3be8cf88debbb7b67214`

This bundle intentionally distinguishes code that is ready to apply from code that is only a future implementation reference. The distinction is required by the repository's own engineering contract: implemented, tested, CI-green, validated and closed are different states.

## PR 8.2 — ready-to-apply source overlay

The `overlay/` directory and `scripts/apply_pr8_2.py` implement the private PHP account-bundle mutation boundary.

Implemented in the bundle:

- multi-section `Gateway_User_Config_File` write boundary for `imap_servers`, `smtp_servers` and `profiles`;
- one exclusive lock, one fresh snapshot, one durable save, fresh multi-section readback;
- private Bridge account bundle `create`, `update`, `test`, and dependency-checked `delete` handlers;
- no persistence during candidate connectivity testing;
- safe deletion that refuses shared IMAP/SMTP references;
- exact default-profile normalization after deletion;
- Bridge capabilities `accounts.test.v1` and `accounts.write.v1` with durable write gating;
- mirrored `cypht-module/gateway` and `modules/gateway` sources;
- PHP dependency/validation tests;
- real `Gateway_User_Config_File` multi-section persistence/conflict test;
- CI/Release test-list wiring via the application script.

Locally verified in this environment:

- `account_handlers.php` parses with PHP 8.4;
- `bridge_accounts.php` parses with PHP 8.4;
- `cypht_account_bundle_persistence.php` parses with PHP 8.4;
- `apply_pr8_2.py` compiles as Python;
- `build_project.sh` passes `bash -n`.

Not verified here:

- application of the transformation script to a local checkout, because this execution environment cannot resolve GitHub for `git clone` / codeload;
- Cypht 2.12.0 / 2.13.0 runtime tests after applying the patch;
- Rust workspace checks after applying the patch;
- GitHub Actions completion on the resulting commit.

Therefore PR 8.2 in this ZIP is **implemented as an exact-base overlay**, but is **not claimed CI-green, validated or closed**.

## PR 8.3+ future reference code

`future-reference/` contains concrete source candidates for later slices. They are deliberately not copied into the active source tree by `apply_pr8_2.py`.

Included reference modules:

- `gateway-auth/account_secrets.rs`
  - in-memory encrypted SecretVault;
  - 300-second default TTL;
  - owner-bound references;
  - one-time consume;
  - fail-closed removal before decrypt;
  - no SQLite persistence.
- `gateway-core/account_api.rs`
  - protocol-neutral create/patch/test/secret request models;
  - password fields are request-only and optional;
  - supports `credential_ref` for MCP-safe flows.
- `gateway-cypht/accounts.rs`
  - Bridge consumer types and account bundle create/update/test/delete calls;
  - separates `accounts.test.v1` from `accounts.write.v1`.
- `gateway-domain/network_policy.rs`
  - fail-closed destination policy;
  - denies loopback, RFC1918, link-local, multicast, unspecified, ULA, CGNAT and benchmarking ranges;
  - explicit CIDR private allowlist;
  - rejects a hostname if any resolved address is forbidden.
- `gateway-storage/operation_journal.rs`
  - durable operation states `claimed`, `in_progress`, `succeeded`, `failed`, `uncertain`;
  - explicit transition API suitable for lost-response handling.

These reference modules still require wiring, migrations, API routing, MCP/CLI integration, repository invariants, and full compile/test review before they are promoted into production code.

## Remaining work before a truthful v1 "closed" claim

The following items are intentionally not represented as complete in this archive:

1. Rust `GatewayService` account lifecycle orchestration, including stable binding hydration for unmanaged pre-existing Cypht accounts.
2. SQLite migration for the operation journal and tests for crash/uncertain outcomes.
3. REST account CRUD + `POST/DELETE /api/v1/account-secrets` routing and OpenAPI schema alignment.
4. Per-operation registry entries and exact Catalog ↔ OpenAPI ↔ Axum ↔ MCP scope invariants for account operations.
5. MCP account tools with schemas that never contain a password field.
6. CLI account create/update/delete/test commands with non-echoed credential entry.
7. DNS resolution and network-policy enforcement immediately before each candidate connection, plus egress-firewall deployment documentation.
8. Idempotency integration for account creates/updates/deletes, including `uncertain` handling after lost Bridge responses.
9. Account health caching; GET must not trigger network probes.
10. Account discovery presets/SRV behavior.
11. IntegrationConnection policy and account allowlist intersection for the new tools.
12. New annotated-tag Release workflow runtime validation.
13. Full repository checks, Cypht 2.13 primary tests, 2.12 compatibility smoke, CI completion, immutable images and coordinated edge promotion on the final SHA.

No item above should be called complete solely because a reference implementation exists in this ZIP.
