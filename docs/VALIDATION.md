## September 25, 2026

### Feeds capability checks

Local Rust tests passed for owner-bound Feed subscription IDs, account-restricted PAT denial (ssert_global_resource_allowed), cross-kind ID rejection, and safe URL mapping. The local Bridge test 	ests/bridge_feeds.php passed for metadata listing and URL/name extraction while ensuring internal server and runtime fields are not leaked. Remote article fetching and subscription mutations remain deferred for SSRF and network egress protection.

All 45 workspace tests passed on Rust 1.94.1 GNU:
- cargo fmt --all -- --check
- cargo clippy --workspace --all-targets -- -D warnings
- cargo test --workspace (45 passed)
- scripts/check-repository.py, scripts/check-version.py, scripts/check-frontend.py all passed.

The local Rust Gateway (gatewayd) is running at http://127.0.0.1:18080/ and the MCP Streamable HTTP server (cypht-mcp) is running at http://127.0.0.1:8790/mcp.
# Validation log

## September 24, 2026

### Sieve capability checks

The Cypht 2.12.0 Bridge contract test passed for redacted Sieve status. The test confirms configured and disabled states without returning a Sieve host or remote credentials. Sieve script writes, ManageSieve probing, and live Unraid status remain unverified.
### Calendar capability checks

Cypht 2.12.0 Calendar source review confirmed a user-level `calendar_events` array with title, description, Unix start time, and simple repeat interval. Local Rust checks cover RFC3339 ranges, opaque calendar/event IDs, global-resource scope denial, and repeat mapping. The local PHP Bridge test covers create, range listing, durable persistence, and confirmation-required deletion. Live Unraid Bridge and provider-backed Calendar behavior remain unverified. Event update, all-day, end time, location, and complex recurrence are intentionally not exposed.
### Saved Searches capability checks

The local Rust tests passed for owner-bound Saved Search IDs, ID retention across successful Gateway renames, ID retirement after deletion and external name changes, account-restricted PAT rejection, Cypht advanced source decoding, and invalid date ranges. The Cypht 2.12.0 Bridge test passed for simple creation, fresh-snapshot locking, rename collision rejection, safe rename, confirmation-required delete, confirmed delete, and disabled-module handling. These are source and local tests. The Unraid Gateway Bridge remains unverified.

### Local MCP read-only and Cypht API login config

The local Rust MCP process listens at `http://127.0.0.1:8790/mcp`. A Streamable HTTP `initialize` and `tools/list` returned HTTP 200. The read-only list contains 18 tools and no write tools. A direct `contacts_create` call returned JSON-RPC error `-32602`. A read call without a bearer token returned an error. No contact or mail data changed.

The Cypht 2.12.0 source does not enable `api_login_key` in its default `config/app.php`. The custom image now loads the value from runtime `API_LOGIN_KEY` through `config/gateway.php`. A PHP 8.1 config-merge test passed. The custom entrypoint rejected a missing key and a key reused for the Bridge. These are local source tests, not a live Unraid login test.

The local `deploy/local-dev.env` still lacks the three Gateway secrets. The running Rust Gateway uses health-only temporary keys. A full local startup failed with a clear missing-settings error. The Unraid Cypht Bridge is still not installed. No end-to-end login or mail read passed.

Current local checks passed with Rust 1.94.1 GNU and a temporary ASCII Cargo target path: `cargo build --workspace`, `cargo test --workspace`, `cargo fmt --all -- --check`, and `cargo clippy --workspace --all-targets -- -D warnings`. PHP 8.1 lint passed for the Bridge and runtime config. PHP 8.1 passed the Bridge contract, persistence guard, Contacts, Tags, Saved Searches, ping, API login config, and encrypted file-settings readback tests. The file-settings test loaded local mbstring and OpenSSL extensions. `scripts/check-version.py`, `scripts/check-repository.py`, `scripts/check-frontend.py`, and the 40-module Cypht matrix check passed with the bundled Python runtime. Ruby parsed OpenAPI, the capability registry, Compose, and both CI workflows. The pinned Rust 1.88 CI toolchain, Docker build, and live Unraid deployment remain unverified.

### Rust Gateway and Cypht HTTP smoke test

The local Rust Gateway runs at `http://127.0.0.1:18080/`. Its configured Cypht upstream is `http://192.168.8.11:8088/`.

| Check | Result |
|---|---|
| Cypht native page | HTTP 200, HTML |
| Gateway `/healthz` | HTTP 200 |
| Gateway `/api/v1/meta/version` | HTTP 200, product version `0.4.0` |
| Gateway `/` | HTTP 200 |
| Unauthenticated `/api/v1/accounts` | HTTP 401 |
| Cypht `?page=process_api_login` probe | HTTP 200, native HTML |
| Cypht `?page=ajax_gateway_ping` probe | HTTP 200, native HTML |

The two Cypht page probes did not return a Bridge JSON envelope. The current Cypht container therefore does not provide a verified `api_login` or Gateway Bridge endpoint.

No mail body, attachment, credential, or provider configuration was read. No mail was sent, moved, or deleted.

### Rust checks

The following checks passed with Rust 1.94.1 GNU on the current computer:

```text
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The pinned Rust 1.88 CI toolchain remains unverified locally.

### Cypht 2.12.0 source checks

The official Cypht v2.12.0 source tarball was downloaded to a temporary directory. Its SHA-256 was `4866c7ab9165b37fb27362f339c43dedd698cb40573c834bb3b75f91deb0260e`; the CI-verified GitHub ZIP archive has SHA-256 `44cf3a67438ed1d0921dbc1cab58a6939b1a09667156984ca193dbce57a24605`.

With `CYPHT_SOURCE_PATH` set to that source, `scripts/check-cypht-matrix.py` passed for all 40 modules and verified the default module list.

A temporary official PHP 8.5.11 CLI bundle ran PHP lint and these Cypht 2.12.0 tests successfully:

```text
tests/bridge_contract.php
tests/bridge_persistence_guard.php
tests/cypht_contacts_repo.php
tests/bridge_ping.php
tests/cypht_tags_repo.php
tests/bridge_tags.php
tests/cypht_saved_searches_repo.php
tests/bridge_saved_searches.php
tests/bridge_calendar.php
```

This local PHP run does not replace the CI PHP 8.1 run.

### Deployment configuration

`docker/Dockerfile.cypht` now retains the Cypht 2.12.0 default modules and adds `api_login,gateway`. Static assertions for the Dockerfile and `deploy/unraid.env.example` passed.

The Dockerfile build, Compose startup, Nginx validation, and Unraid image installation remain unverified because Docker is not installed on this computer.

A batch-mode SSH probe to `root@192.168.8.11` failed with public-key and password authentication denied. No remote container or filesystem changes were attempted.

## September 23, 2026

### Unraid inspection

Read-only SSH inspection confirmed a healthy `cypht/cypht:2.12.0` container on x86_64. The container publishes host port `8088` to container port `80`.

The inspection found these persistent mounts:

| Host path | Container path |
|---|---|
| `/mnt/cache/appdata/cypht/attachments` | `/var/lib/hm3/attachments` |
| `/mnt/cache/appdata/cypht/app_data` | `/var/lib/hm3/app_data` |
| `/mnt/cache/appdata/cypht/users` | `/var/lib/hm3/users` |

Cypht uses Docker's default `bridge` network. The app database uses MySQL. No Gateway container or Gateway Bridge directory was present. The Unraid template did not expose a `CYPHT_MODULES` parameter. The generated module list and saved account/profile state were not verified.

Host ports `8089` and `18080` were free during that inspection. Recheck them before deployment.

### Earlier local smoke test

The local Gateway health, version, and UI routes returned HTTP 200. An unauthenticated account request returned HTTP 401. The MCP initialize and tool-list requests succeeded. An MCP account call without a bearer token was rejected.

## Not verified

- Cypht 2.12.0 Bridge ping and version handshake through the real container.
- Rust Gateway login through `api_login`.
- Accounts, mailboxes, search, message, and attachment reads through the Bridge.
- Contacts and Tags durable writes, concurrency, and fresh-session readback.
- Docker image builds and Compose startup.
- Unraid Gateway deployment or port cutover.
- GitHub Actions execution and GHCR publication.
- Rust 1.88 CI results.
- Sending profile availability and all mail write operations.
