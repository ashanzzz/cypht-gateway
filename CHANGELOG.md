# Changelog

All notable changes to this project will be documented here. The project follows Semantic Versioning.

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
