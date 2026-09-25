# Cypht 2.12.0 module-to-Gateway map

## Evidence and scope

This map uses the official Cypht `v2.12.0` source archive reviewed for this project. The archive SHA-256 is `44CF3A67438ED1D0921DBC1CAB58A6939B1A09667156984CA193DBCE57A24605`. A module listed here may be disabled in the Unraid container. This source map does not prove that a module is installed, enabled, configured, or safe at runtime.

Cypht 2.12.0 source defaults `CYPHT_MODULES` to `core,contacts,local_contacts,feeds,imap,smtp,account,idle_timer,calendar,themes,nux,developer,history,saved_searches,advanced_search,highlights,profiles,inline_message,imap_folders,keyboard_shortcuts,tags,brute_force`. The default excludes `api_login`. The Gateway Bridge is a custom module and must be added to the deployed module list.

`Mapped` means that the Gateway has a corresponding public capability. `Partial` means only part of the native module is exposed or the live integration is not verified. `Deferred` means the Gateway does not expose it yet. `Excluded` means the capability stays outside the public API for its current risk or product scope. `Infrastructure` means Cypht owns the behavior and the Gateway does not expose it as a separate capability.

## Upstream modules

| Cypht module | Gateway mapping | Status and boundary |
|---|---|---|
| `2fa` | None | Excluded. Do not manage second-factor enrollment or recovery through the Gateway. |
| `account` | `auth.login`, `account.list` | Partial. Password changes, account provisioning, and account deletion are excluded. |
| `advanced_search` | Simple search execution only | Partial. The REST `/messages/search` operation does not execute Cypht advanced searches. Saved advanced metadata is stored and managed by the `saved_searches` capability. |
| `api_login` | `auth.login` | Mapped through Cypht's API-login mechanism. Runtime authentication and rate-limit parity still require deployment tests. |
| `brute_force` | None | Infrastructure. Keep Cypht's protection enabled. Confirm API-login requests receive the intended protection before deployment. |
| `calendar` | `/api/v1/calendars` and event routes | Partial. Cypht 2.12.0 simple user events support list/create/delete and day/week/month/year expansion. No update, all-day, end time, location, or complex recurrence. Live Bridge tests remain unverified. |
| `carddav_contacts` | None | Deferred. External CardDAV access and credential handling are not exposed. |
| `contacts` | `/api/v1/contacts` | Partial. The custom file adapter passed local persistence and readback tests. Live Unraid writes and DB-backed settings are unverified. |
| `core` | Sessions, settings load, and shared handlers | Infrastructure. Cypht owns user configuration, sessions, and common request behavior. |
| `desktop_notifications` | None | Excluded. Browser push and notification subscriptions are not part of the API. |
| `developer` | None | Excluded. Do not expose server configuration or diagnostic details. |
| `dynamic_login` | None | Infrastructure. Cypht owns provider login and OAuth flows. Rust does not reimplement OAuth. |
| `feeds` | `/api/v1/feeds`, `/api/v1/feeds/{id}` | Partial. Read-only subscription metadata is exposed. Remote article fetching and feed mutations remain deferred for SSRF and egress protection. |
| `github` | None | Excluded from mail Gateway parity. Cypht's GitHub activity integration is not a mail capability. |
| `hello_world` | None | Excluded. This is an upstream example module. |
| `highlights` | None | Deferred. UI highlight preferences are not in REST v1. |
| `history` | None | Deferred. The Gateway does not expose the native multi-provider History UI store. |
| `idle_timer` | None | Infrastructure. Cypht owns browser idle-session behavior. |
| `imap` | Account, mailbox, message, search, attachment, and mail-write APIs | Partial. Cypht remains the protocol engine. The Gateway exposes only reviewed operations and opaque IDs. |
| `imap_folders` | Mailbox list | Partial. Folder list is exposed. Folder creation, deletion, subscription, and management are deferred. |
| `inline_message` | Message read API | Partial. The Gateway returns its reviewed message representation, not Cypht's inline-rendering preferences. |
| `keyboard_shortcuts` | None | Deferred. Shortcut settings and key bindings are not in REST v1. |
| `ldap_contacts` | None | Deferred. LDAP lookup and bind credentials are not exposed. |
| `local_contacts` | `/api/v1/contacts` | Partial. Local CRUD/search and file persistence tests pass. Live Unraid write conflicts and readback remain unverified. |
| `mta_sts` | None | Infrastructure. Cypht owns SMTP transport policy. The Gateway does not override it. |
| `nasa` | None | Excluded from mail Gateway parity. This is an upstream content integration. |
| `nux` | None | Excluded. Onboarding and service-discovery UI are not public API operations. |
| `pgp` | None | Deferred. Key import, signing, encryption, and private-key handling need a separate security design. |
| `profiles` | `GET /api/v1/profiles` | Partial. Read/list support exists. Profile creation, update, and deletion are deferred. |
| `recaptcha` | None | Infrastructure. Cypht owns browser login challenges. API-login behavior needs a live security test. |
| `recover_settings` | None | Excluded. Account recovery and encrypted-settings recovery are not exposed. |
| `saved_searches` | `/api/v1/saved-searches` | Partial. Simple and advanced metadata CRUD is mapped across REST, CLI, MCP, and UI. Gateway renames preserve encrypted, owner-bound opaque IDs. Account-restricted PATs are denied. Advanced source references map to opaque account and mailbox IDs. Local source tests pass; live Unraid Bridge tests remain unverified. |
| `scheduled_sends` | `schedule_at` on mail write | Partial. The Bridge stores scheduled messages through Cypht when the module is enabled. Scheduler behavior and send outcomes are not verified on Unraid. |
| `sievefilters` | `/api/v1/sieve/status` | Partial. Redacted per-account configuration status is exposed. Scripts, filters, credentials, capabilities, remote probing, and writes are deferred. |
| `site` | None | Infrastructure. Site authentication, configuration, and headers remain Cypht-owned. |
| `smtp` | Send and draft APIs | Partial. Cypht owns SMTP. No live sending profile has been verified, and no test mail was sent. |
| `tags` | `/api/v1/tags` and message-tag operations | Partial. Account-scoped code and local file persistence tests pass. Live Unraid writes and DB-backed settings are unverified. |
| `themes` | None | Deferred. Theme selection and custom theme settings are not in REST v1. |
| `wordpress` | None | Excluded from mail Gateway parity. Cypht's WordPress integration is not a mail capability. |
| `gmail_contacts` | None | Deferred. Google Contacts credentials and external contact lookup are not exposed. |

## Gateway-owned module

The repository's `cypht-module/gateway` directory is not an upstream Cypht module. It provides private Bridge pages for Rust. Every page must require an authenticated Cypht session and `X-Cypht-Gateway-Key`. Public clients must use REST `/api/v1`; they must never call Bridge pages directly.

## Release rule

This table describes source-level mapping only. Do not promote `Partial` or `Deferred` rows based on route existence. Promote a row only after its permissions, persistence, audit, tests, enabled-module behavior, and Unraid integration are verified. See `docs/PARITY.md` for the release ledger.
