# Cypht compatibility

## Unraid runtime baseline

The supported runtime for this Gateway work is Cypht `2.12.0` in Unraid Docker. Cypht native UI remains at `http://192.168.8.11:8088/`. Deploy Gateway on a separate host port. When both containers share a Docker network, use a private Cypht service URL such as `http://cypht:80/`; `localhost` inside the Gateway container does not name Cypht.

The official Cypht `v2.12.0` GitHub ZIP tag archive used for review has SHA-256 `44CF3A67438ED1D0921DBC1CAB58A6939B1A09667156984CA193DBCE57A24605`. The separately downloaded source tarball used for local PHP checks has SHA-256 `4866c7ab9165b37fb27362f339c43dedd698cb40573c834bb3b75f91deb0260e`. It defines `CYPHT_VERSION` as `2.12.0`. The earlier v2.12.2 audit remains in `docs/CYPHT_V2_12_2_AUDIT.md` as historical evidence, not the runtime compatibility contract. The user's v2.12.2 Docker login failure is not reproduced here, and upgrading to that version is not required by this goal.

The Bridge uses Cypht's `api_login`, `Hm_IMAP_List`, `Hm_Mailbox`, `Hm_SMTP_List`, `Hm_Profiles`, `Hm_MIME_Msg`, and AJAX-page APIs. In the official 2.12.0 source, `api_login_key` is not enabled by the default `config/app.php`; the custom image therefore adds it through `config/gateway.php` from runtime `API_LOGIN_KEY`. The 2.12.0 Contacts repository CRUD test passes with the official source. A file comparison found the `api_login`, Contacts repository, SMTP, Feeds, Calendar, Tags, and Sieve library files used here identical between v2.12.0 and v2.12.2. This is not a full upstream audit.

Two mail-core changes matter for v2.12.0. Its IMAP APPEND uses `mb_strlen($msg)` where the later source uses `strlen($msg)`. Non-ASCII draft or Sent-folder saves may fail because IMAP needs a byte count. Its `append_end()` also lacks the later fallback when APPENDUID is absent or malformed. Test these paths on isolated mail before claiming draft and scheduled-send parity. Do not silently replace Cypht's mail engine in Rust.

## v2.12.0 risk register

These are static source findings, not live exploit reports. Keep the Unraid instance on a trusted network while they remain unresolved.

- **Unsafe logout input:** `modules/core/handler_modules.php` decodes a request value with `unserialize()`. Later source uses scalar-only JSON. Test and backport the encoder, decoder, and logout call sites together. Do not claim code execution without a separate proof.
- **Native attachment paths:** `modules/smtp/modules.php` builds paths from upload names before the later basename and directory checks. Test native chunk upload, send, and draft with traversal and Unicode filenames. The Gateway Bridge has a separate upload handler, but the Cypht UI remains in scope.
- **Native Contacts display:** `modules/contacts/site.js` inserts contact text into HTML without the later escaping. Test untrusted names in the native popup and backport output escaping before wider exposure.
- **IMAP APPEND result:** a failed APPEND can return a truthy response array. Later source returns `false` on failure or `true` when success has no usable UID. The Bridge now treats array-valued draft results as uncertain and does not publish an array as a message UID. This prevents a false confirmed result but cannot tell a failed APPEND from an accepted APPEND without UIDPLUS. Test `NO`/`BAD` replies and success without UIDPLUS before enabling Gateway drafts.
- **IMAP literal length:** v2.12.0 uses character count for an IMAP byte length. Test non-ASCII drafts and Sent copies. Never resend SMTP solely because saving a Sent copy fails.

Keep any security backports traceable to the official source diff. Build a 2.12.0-based custom image and verify login, native UI, and Gateway Bridge before deploying it. The version constant must remain `2.12.0`; a custom backport is not proof of full v2.12.2 compatibility.

## Saved Searches repository semantics

Cypht 2.12.0 stores simple and advanced Saved Searches in a user-config map keyed by name. Its `Hm_Saved_Searches::rename()` overwrites an existing destination key. The Gateway checks the destination under the durable user-config write lock, then changes the map without calling that method. A Bridge contract test verifies that a collision does not overwrite the target.

Advanced Cypht metadata includes `sources[].source` values with internal IMAP account IDs and hex-encoded folder names. The Gateway maps these values to version-2 opaque account and mailbox IDs in REST, CLI, MCP, and UI responses. Local source tests cover source decoding and malformed identifiers. Live provider and Unraid behavior remain unverified.
## Calendar compatibility

Cypht 2.12.0 Calendar stores a user-level `calendar_events` array with `title`, `description`, `date`, and simple repeat interval. Its event ID is a content-derived MD5 and changes when fields change. The Gateway does not expose that ID. It provides a synthetic user calendar, opaque event IDs, RFC3339 range reads, create, and confirmed unique delete. Event update and fields absent from Cypht remain deferred. Live Bridge behavior is unverified.
## Tags repository scope

Cypht 2.12.0 `Hm_Tags::removeMessage()` removes a matching UID from all servers and folders attached to a tag. UIDs can repeat across accounts. The Gateway Bridge instead edits one account and folder entry, and a repository test covers two accounts with the same UID. The REST domain denies account-restricted PATs access to global Tags. Gateway MOVE and archive now synchronize Tags when Cypht returns a verified new UID. When it does not, the Bridge keeps the old association and reports `tag_sync: pending` rather than guessing. Permanent deletion removes a scoped Tag association; a Trash move without a new UID stays pending. These paths passed mock Bridge tests, not live Unraid provider tests.
## Version handshake

The private Bridge ping returns `CYPHT_VERSION` and the Bridge product version. The Rust adapter accepts exactly Cypht `2.12.0` and the matching Gateway/Bridge product version. It rejects unknown or unreviewed upstream versions with a bounded capability-unavailable error. It never returns Cypht credentials. A version match does not prove an optional module is enabled or a provider is reachable.

## Deployment and security boundaries

Keep the Bridge private. Require both a Cypht session and `X-Cypht-Gateway-Key` for every Bridge page. Store `API_LOGIN_KEY` and `GATEWAY_BRIDGE_KEY` as separate Unraid container secrets or protected environment values. Do not put them in Git, test reports, logs, or public API responses.

The pinned Cypht `2.12.0` source defaults `CYPHT_MODULES` to `core,contacts,local_contacts,feeds,imap,smtp,account,idle_timer,calendar,themes,nux,developer,history,saved_searches,advanced_search,highlights,profiles,inline_message,imap_folders,keyboard_shortcuts,tags,brute_force`. The Unraid container and its template do not set a module-list variable. This supports the expected defaults but does not replace a runtime module check. The custom Gateway image must retain the defaults and add `api_login,gateway`.

An earlier local browser session used `localhost:8088`; this is not the Unraid service address. That session showed one readable IMAP account and one SMTP configuration. A different browser session at `http://192.168.8.11:8088/` showed no accounts, so a fresh authenticated LAN session must confirm the saved configuration. No sending profile was observed. Read-only SSH inspection confirmed the healthy Cypht 2.12.0 container, its `8088 -> 80` mapping, the three persistent bind mounts listed in `docs/UNRAID.md`, and the default Docker bridge network. It also confirmed a MySQL backend, no `CYPHT_MODULES` environment variable, no Gateway container, and no installed Gateway Bridge path. The generated module list and saved account/profile state remain unverified. Unraid host port `8080` is occupied. Unraid host port `18080` was free during the September 23 inspection, but it must be checked again before deployment. The local development Gateway uses port `18080` on the current computer only.

Using an older upstream release requires explicit security review and network limits. Do not assume v2.12.0 includes fixes introduced in later releases. Restrict Cypht and Gateway exposure to trusted networks or an authenticated TLS reverse proxy. Never expose Bridge pages as a public service.
## Contacts and Tags persistence

The stock Cypht 2.12.0 user-config classes do not prove durable Contacts and Tags writes. The custom file-settings adapter adds a capability marker, per-user locks, revision checks, atomic replacement, and fresh-load readback. Local encrypted and unencrypted file tests pass, including stale-writer and wrong-key rejection. The Bridge fails closed for database-backed user settings or a missing adapter. The adapter has not passed live Unraid or simultaneous native-UI/Gateway write tests. Do not enable real-data writes until those checks pass.
