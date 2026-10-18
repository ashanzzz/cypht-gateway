# Changelog

All notable changes to this project will be documented here. The project follows Semantic Versioning.

## [Unreleased] - 2026-10-17

### Account Lifecycle Core Models & Migration v2 (PR 8.1)
- **Protocol-Neutral Account Models (`crates/gateway-core`)**: Established `ManagedAccount`, `AccountEndpoint`, `AccountSecurity`, `AccountIdentity`, and `AccountHealth` representing public account lifecycle states without leaking Cypht-internal implementation IDs into the public contract.
- **Storage Migration v2 (`crates/gateway-storage`)**: Upgraded `CURRENT_SCHEMA_VERSION` to 2 with transactional DDL introducing `gateway_account_bindings` (with active uniqueness and retirement semantics) and `gateway_account_bundles` (mapping managed accounts to Cypht IMAP/SMTP/profile resources).
- **Ephemeral Secret Hygiene**: Deliberately excluded transient 300s account password references from persistent SQLite schema to prevent disk residue, reserving secret handoff for in-memory encrypted storage in PR 8.3.
- **Granular Account Scope Specification**: Updated `docs/ACCOUNTS_PLAN.md` with refined scopes (`accounts.credentials.write`, `accounts.test`, `accounts.delete`) and canonical `/api/v1/account-secrets` endpoint path.

### Versioned Storage Migrations
- **Versioned SQLite Migration Subsystem**: Introduced `crates/gateway-storage/src/migrations.rs` managing schema state via `PRAGMA user_version` and atomic transaction execution, establishing a solid migration boundary before expanding account and secret models.
- **Backwards-Compatible Baseline v1**: Replaced raw ad-hoc `CREATE TABLE IF NOT EXISTS` execution in `Store::open()` and `Store::open_memory()` with atomic version-1 baseline migration, seamlessly preserving data in existing unversioned databases.

### Operation Catalog & Tool Annotations (PR 6)
- **Unified Operation Catalog (`api/capability-registry.yaml`)**: Extended capability registry with granular `backend_capabilities` mappings and 5 standardized ToolAnnotation presets (`read`, `write`, `update`, `destructive`, `send`), establishing a single source of truth without framework codegen overhead.
- **Bi-directional MCP Catalog Invariant**: Added missing `mail_list_attachments`, `feeds_list`, and `feeds_get` to the catalog. Enforced exact bi-directional set equality in `scripts/check-repository.py` between catalog MCP tools (42) and MCP source tools (42).
- **Exact Risk & Annotation Consistency**: Enforced that `WRITE_TOOLS` gate matches catalog write/destructive risks exactly, and that every tool annotation strictly matches its operation risk profile.
- **Wire-Level MCP ToolAnnotations**: Implemented `annotations(...)` metadata across all 42 `#[tool]` definitions in `crates/gateway-mcp/src/lib.rs`, exposing `read_only_hint`, `destructive_hint`, `idempotent_hint`, and `open_world_hint` directly on the wire in MCP `tools/list`.
- **Exact HTTP Method & Annotation Validation**: Enhanced `scripts/check-repository.py` to structurally validate both path and HTTP method against parsed OpenAPI YAML and the Axum router, and mechanically assert that every MCP tool's wire annotation hints match its catalog preset.
- **Durability Policy Decoupled from Transport**: Shifted `user_config.durable.v1` validation into `post_user_config_with_capability` at the capability layer, removing business durability logic from raw bridge POST transport helpers.

### Build & CI Fixes
- **P0 CI Dependency Fixes**: Added `tracing.workspace = true` to both `crates/gateway-cypht/Cargo.toml` and `crates/gateway-domain/Cargo.toml`, resolving module resolution errors and container build failures in GitHub Actions.
- **Granular Capability Enforcement**: Split capability contracts into explicit `.read.v1` and `.write.v1` domains across both PHP Bridge and Rust adapter. Introduced private checked primitives (`get_with_capability`, `post_user_config_with_capability`, `write_with_capability`) in `gateway-cypht`, guaranteeing 100% adapter operation coverage without developer omission.
- **Container Workflow Tag Protection**: Restricted `cypht-gateway-aio:latest` tag publication in `.github/workflows/image.yml` to official SemVer release tags, preventing accidental overwrite on push to `main`.

### Upstream Baseline & Security Hardening
- **Cypht 2.13.0 Production Baseline**: Upgraded `Dockerfile.cypht`, `Dockerfile.aio`, and CI workflow container builds to Cypht `v2.13.0` (incorporating critical SMTP attachment and chunk isolation fixes).
- **SSO PAT Re-authentication Barrier (P0-06)**: Introduced `AuthKind::SsoSession` to prevent passwordless native SSO sessions from minting long-lived PATs with empty credentials, requiring explicit re-authentication.
- **Vault Secret Buffer Zeroization (P0-07)**: Wrapped transient plaintext encryption and decryption buffers in `Zeroizing<Vec<u8>>` to eliminate lingering secret residue in heap memory.
- **Capability Handshake**: Introduced structured `capabilities` array and `bridge_api_version` in Bridge Ping responses, enabling feature detection over rigid version checks.
- **Credential Debug Redaction**: Implemented custom `std::fmt::Debug` for `Credential` and `CyphtSession` to mask sensitive passwords and session tokens (`[REDACTED]`) from application logs and traces.
- **P0 Fail-Closed PAT Deserialization**: Fixed a security vulnerability in `gateway-storage` where corrupted `account_allowlist_json` or `scopes_json` silently defaulted to `[]` via `unwrap_or_default()`, inadvertently escalating restricted PATs to full access across all accounts. Implemented strict `json_column()` deserializer that fails closed on invalid JSON.
- **SSO Multi-Container & Edge Protection**: Replaced hardcoded `127.0.0.1:18080` in Native SSO with configurable `GATEWAY_INTERNAL_URL` for container network compatibility, and blocked `/api/v1/auth/sso` from the public edge proxy.
- **Audit Failure Resilience**: Decoupled audit log persistence errors from domain write outcomes, logging warnings rather than falsely failing external mutations that have already succeeded.
- **Engineered Execution Plan for Mail Accounts (v0.6.0)**: Added `docs/ACCOUNTS_PLAN.md` with dependency-checked bundle deletion and secret reference pattern for MCP tools.

### Engineering Discipline & Repository Invariants
- **Codified Engineering Discipline in AGENTS.md**: Enforced strict rules governing language boundaries, preservation of domain semantics over visual shape, capability contract requirements, evidence-based status claims, and antagonist review steps.
- **Bidirectional PHP Mirror Guard**: Upgraded `scripts/check-repository.py` to perform bidirectional set comparison between `cypht-module/gateway` and `modules/gateway`, catching unexpected extra files as well as missing ones.
- **Behavioral Capability Test Suite**: Added `tests/bridge_capabilities.php` to mechanically test module-present/module-missing behavior and the complete durable write-capability set in both fail-closed and storage-safe states.
- **Coordinated Image Promotion & Immutable Bundle Identity**: Container publication now starts only after successful CI on `main`, uses non-overwriting full-SHA identities for Gateway, Cypht Bridge, and AIO, and moves `edge` only from a complete bundle. Release no longer rebuilds containers; it verifies the same SHA bundle, exclusively owns release aliases, and attaches `image-bundle.json`. Cross-package alias updates are explicitly coordinated rather than described as atomic.
- **Cypht 2.13 Primary Source Validation**: CI and Release run the full Bridge/repository suite against pinned Cypht `v2.13.0` commit `4caf0a64a4d4b1b2756ea05e3883aacac16babc2`; checksum-pinned Cypht `v2.12.0` remains a compatibility smoke baseline.
- **Clarified OAuth Invariants in AGENTS.md**: Disambiguated provider mailbox OAuth flows (owned by Cypht) from Gateway-native OAuth authorization server features (owned by Rust control plane).
- **Automated Capability Producer-Consumer Checker**: Enhanced `scripts/check-repository.py` to mechanically verify that every capability constant consumed by Rust is advertised by the PHP Bridge, preventing broken capability contracts.
- **PHP Mirror Drift Guard**: Added automated zero-drift verification in `scripts/check-repository.py` between `cypht-module/gateway` and `modules/gateway`.
- **Feeds Capability Advertising**: Dynamically advertised `feeds.read.v1` in PHP `Hm_Handler_gateway_ping` when the Cypht `feeds` module is supported.
- **Accurate Mailbox Read Capability**: Re-mapped `mailboxes()` in `gateway-cypht` to `CAP_MAIL_READ` to align with IMAP folder retrieval semantics.

### Performance & Tooling
- **PAT Last-Used Throttling**: Throttled `touch_pat` SQLite updates to once every 60 seconds, eliminating redundant WAL writes on frequent API/MCP polling.
- **Optimized Repository Verification**: Excluded the `target/` build directory from recursive secret scans in `scripts/check-repository.py`, reducing local check execution time from >20s to <1.5s.

## [0.5.2] - 2026-09-26 (Official Production Release)

### Native Cypht Webmail Integration & UI Parity
- **Official Upstream Merge**: PR #2110 merged into `cypht-org/cypht` master (visual icon parity across server and account headers).
- **Zero-Iframe Native DOM UI**: Rewrote `Hm_Output_gateway_page_content` to render 100% native Bootstrap 5 DOM, completely eliminating iframe white flashes, double scrollbars, and CSP framing issues.
- **Seamless Single Sign-On (SSO)**: Implemented `POST /api/v1/auth/sso` and `Hm_Handler_gateway_sso_data` for automatic loopback session exchange, allowing logged-in Cypht users to enter the Gateway console instantly without re-entering credentials.
- **All 20 Permission Scopes & Presets**: Provided one-click scope preset buttons ("All Scopes / 全选", "Standard Read/Write", "AI Assistant", "Clear") with clear categorized bilingual descriptions.
- **Client Lifecycle via `site.js`**: Structured client-side code into `modules/gateway/site.js` using Cypht's canonical `applyGatewayPageHandlers()` lifecycle, integrating smoothly with the SPA router.
- **Self-Healing Container Build**: Automated `php scripts/config_gen.php` compilation in `Dockerfile.aio` and `all-in-one-entrypoint.sh` to produce fresh production `site.js` with cache busting on every startup.

### REST API, MCP & Resilience
- **Resilient Unified Inbox**: Replaced hard-failing iteration in `messages()` with graceful per-account handling, ensuring that temporary provider connectivity issues do not block other healthy mail accounts.
- **Streamable HTTP MCP Server**: Validated `/mcp` endpoint against Model Context Protocol (2024-11-05 specification) with Bearer token authentication and default read-only safety policy.
- **Opaque v2 Resource IDs**: Enforced HMAC-SHA256 owner-, type-, and source-bound opaque IDs across all entities.

- Added read-only RSS/Atom feed subscriptions metadata through the Bridge, REST, CLI, MCP, and management UI. Article fetching and feed mutations remain deferred for SSRF and network egress protection.
- Added redacted per-account Sieve status across the Bridge, REST, CLI, MCP, and UI. Sieve scripts, filters, credentials, remote probing, and writes remain deferred.
- Added the limited Cypht 2.12.0 Calendar capability for user-level list, create, range, and delete across Bridge, REST, CLI, MCP, and UI. Event update and unsupported calendar fields remain deferred.
- Added Saved Searches metadata CRUD through the Bridge, REST, CLI, MCP, and management UI. Stable opaque IDs survive Gateway renames, account-restricted PATs are denied, and advanced sources use opaque account/mailbox IDs. Live Unraid verification remains pending.

- Reserved object-ID kinds and read/write scopes for contacts, tags, saved searches, calendars, and feeds.
- Added owner-, kind-, and source-bound v2 resource IDs for accounts, mailboxes, messages, and attachments. Encrypted lookup mappings support stable resolution, while the signed v1 decoder remains for migration.
- Added a read scope for Sieve status. The default AI token preset contains no new write scope and grants no unimplemented read capability.
- Hid MCP write tools from `tools/list` when write mode is off. Direct calls also fail until runtime opt-in and a scoped PAT are present.
- Added CI builds for the Gateway and pinned Cypht Bridge container images. Release tags publish versioned Gateway multi-platform and pinned Bridge AMD64 images to GHCR.
- Updated the Cypht 2.12.0 Bridge image defaults to retain all source modules and enable api_login and gateway.
- Fixed the Cypht 2.12.0 image to load `API_LOGIN_KEY` through a runtime config file; the key is no longer silently ignored by the stock `config/app.php`.
- Extended version updates to keep `VERSION`, Cargo, and the Bridge version file in sync. Added a release checklist for annotated SemVer tags.
- Added an optional edge proxy for Gateway UI `/`, REST `/api/v1`, and MCP `/mcp` on one host port. The Unraid cutover remains unverified.
- Mapped an explicit bridge HTTP 501 response to `capability_unavailable` instead of an upstream failure.
- Added local Contacts CRUD and search through the private Cypht bridge, REST, CLI, MCP, and management UI. Live deployment is not yet verified.
- Added Tags CRUD and message association across local Bridge, domain, REST, CLI, MCP, and UI. Tag removal is scoped to account and folder to avoid Cypht 2.12.0 UID collisions. Live deployment remains unverified.
- Normalized boolean and array Cypht message-action results. Gateway MOVE/archive synchronize Tags only with a verified destination UID and report `tag_sync: pending` otherwise.
- Fixed the Cypht request-header allowlist, saved the session before Bridge responses, and bounded attachment downloads to 25 MiB.
- Fixed Rust compilation errors in the domain, core exports, and MCP adapter.
- Added management UI guidance for an empty Cypht account list or missing sending profile.
- Bound Compose REST to loopback by default and added a configurable candidate host port (`GATEWAY_HOST_PORT=18080`). The port is not yet verified on Unraid.
- Added a custom Cypht file-settings adapter with per-user locks, revision checks, atomic writes, and fresh-load readback for Contacts and Tags. The stock and database-backed settings paths still fail closed.
- Passed local Cypht 2.12.0 file-settings persistence and stale-writer tests. Real Unraid Bridge writes remain unverified.
- Aligned the private version handshake and Docker image with the reviewed Cypht 2.12.0 Unraid runtime. Documented older-upstream risks and pinned source tests.
- Stopped the 2.12.0 Bridge from treating array-valued IMAP APPEND results as verified draft UIDs; SMTP success remains distinct from Sent-copy uncertainty.

## [0.4.0] - 2026-09-22

### Added

- `cypht-mcp` binary with stdio and MCP Streamable HTTP transports.
- AI-safe mail tools covering accounts, profiles, mailboxes, list/search/read, attachment metadata/download/upload, draft/send/reply/forward, message mutations and audit.
- `audit.read` scope with user-scoped REST, CLI, MCP and management-UI audit access.
- Request correlation IDs on public API responses and structured errors.
- MCP binary in Docker images, Compose profile and GitHub release artifacts.
- Repository invariant checking for registry-declared MCP tools.

### Changed

- Rust MSRV raised to 1.88 for the MCP SDK.
- MCP starts read-only; write tools require explicit `--allow-write`.
- CLI send/draft/reply/forward require a caller-supplied stable idempotency key.
- Browser compose retries reuse one idempotency key until success/reset.

### Security

- Streamable HTTP MCP uses the request bearer PAT rather than a shared privileged identity.
- MCP message/attachment output is labeled untrusted external content; message HTML is omitted from `mail_read`.
- MCP attachment download/upload limits reduce accidental model-context and memory abuse.
- Audit output remains metadata-only and user-scoped.

## [0.3.0] - 2026-09-22

### Added

- Sending-profile API with opaque profile IDs and account allow-list enforcement.
- Temporary outgoing attachment upload and authenticated attachment download.
- Send-now and scheduled-send APIs using Cypht's SMTP, OAuth refresh, MIME and Sent-folder logic.
- Draft creation through Cypht mailbox storage.
- Reply, reply-all and forward APIs.
- Read/unread and flag/unflag updates.
- Move, archive and trash/delete operations.
- 24-hour SQLite idempotency records for send, draft, reply and forward.
- Management UI compose flow, attachment transfers and message action controls.
- Matching `cyphtctl` write commands.
- Expanded OpenAPI and capability registry for all v0.3 mail operations.

### Security

- Public upload IDs are signed and bound to the authenticated gateway user.
- Send-capable profiles are filtered through PAT account allow-lists.
- Destructive delete remains isolated behind the `mail.delete` scope.
- Outgoing temporary attachment files are encrypted with Cypht's request key, use mode `0600`, and are cleaned up after successful use or the stale-file TTL.
- External sends atomically claim idempotency keys before SMTP; concurrent duplicates are blocked and keys cannot be reused with a different request body.

## [0.2.0] - 2026-09-22

### Added

- Private Cypht PHP bridge protected by Cypht session authentication and a separate bridge key.
- Cypht `api_login` integration for gateway login.
- Encrypted XChaCha20-Poly1305 credential/session vault backed by SQLite.
- Short-lived gateway access tokens and persistent PATs with scopes, expiry, revocation and account allow-lists.
- HMAC-signed opaque account, mailbox, message and attachment IDs.
- Account and mailbox APIs.
- Single-account and unified inbox APIs.
- Cross-account search API.
- Parsed message reading with attachment metadata.
- Functional management UI for login, PAT management, accounts, inbox, search and message reading.
- Expanded `cyphtctl` commands, including identity and PAT creation/revocation.
- Account allow-list selection in the management UI.
- Exact gateway/bridge version handshake.
- Capability registry and AI maintainer contract.
- Security and Cypht compatibility documentation.

### Security

- Provider credentials and OAuth tokens are never exposed by the bridge.
- Default AI PAT scopes remain read-only.
- Persistent Cypht credentials are encrypted with a deployment master key and never stored as plaintext.
- Separate derived keys are used for encryption, PAT hashing and opaque-ID signing.
- PAT token-management delegation cannot broaden scopes or account access.
- Restricted searches return an empty result rather than falling back to all accounts when no allowed account remains.

## [0.1.0] - 2026-09-22

### Added

- Initial Git repository and SemVer policy.
- Rust workspace foundation.
- Build metadata with Git SHA, tag and dirty state.
- Health and version HTTP endpoints.
- Initial `cyphtctl` CLI.
- OpenAPI seed document.
- Docker and Docker Compose foundation.
- GitHub CI, GHCR image build and tagged release workflows.
