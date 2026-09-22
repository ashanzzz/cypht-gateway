# Changelog

All notable changes to this project will be documented here. The project follows Semantic Versioning.

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
