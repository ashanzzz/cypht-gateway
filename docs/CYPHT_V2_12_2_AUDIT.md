# Cypht v2.12.2 comparison audit

Historical audit: the current Unraid runtime target is Cypht 2.12.0. See `docs/CYPHT_COMPATIBILITY.md` for the active contract. Statements below describe the earlier v2.12.2 review and must not be read as current deployment requirements.

Date: 2026-09-23

## Evidence and limits

- Source: the official GitHub `cypht-org/cypht` `v2.12.2` tag archive from `codeload.github.com`.
- Archive SHA-256: `DAA168A9C50B34050BC56D4049509F5BDB00C5595A55096E6B81D1FBD653715A`.
- The archive defines `CYPHT_VERSION` as `2.12.2` in `lib/version.php:3`.
- This is a static source comparison. No Cypht server, mailbox, PHP runtime, or live provider was tested.
- Paths prefixed `upstream/` below refer to that tag. Paths prefixed `gateway/` refer to this repository.

## Findings in the restored v0.4 baseline

### Blocker: the private bridge key header is filtered out

Cypht copies only `allowed_server` fields into `$request->server` (`upstream/lib/request.php:98-102`). Its core filter does not include `HTTP_X_CYPHT_GATEWAY_KEY` (`upstream/modules/core/setup.php:266-287`). The gateway filter adds no `allowed_server` entries (`gateway/cypht-module/gateway/setup.php:64-79`). The guard reads the missing field from `$this->request->server` and returns HTTP 403 (`gateway/cypht-module/gateway/handler_modules.php:7-15`). This blocks all bridge pages, including ping. Cypht also filters out the upload filename and content-type headers and `CONTENT_LENGTH`.

Fix: add the exact gateway headers and `CONTENT_LENGTH` to the gateway module's `allowed_server` filter. Test a ping and an upload through the real Cypht request filter.

### High: JSON helpers stop Cypht before session save

Both gateway JSON helpers call `Hm_Functions::cease()` (`gateway/cypht-module/gateway/functions.php:10-25`). In Cypht this method calls `die()` (`upstream/lib/framework.php:96-98`). The normal dispatcher saves the session later (`upstream/lib/dispatch.php:230,268-275`). The upload handler writes `gateway_uploads` only to its in-memory session and then calls the JSON helper (`gateway/cypht-module/gateway/handler_modules.php:200-215`). A second request may not find the uploaded file's ID. Future contact, calendar, and feed writes have the same persistence risk.

Fix: return through Cypht's dispatch lifecycle, or save and close the session before ending a bridge response. Verify session persistence with both PHP and DB-backed sessions.

### High: the current signed IDs are readable, not opaque

`ObjectIdCodec::encode` base64-encodes JSON that contains raw `parts` (`gateway/crates/gateway-core/src/ids.rs:48-67`). A client can decode the first token segment without the signing key. Mailbox names, message UIDs, and future contact internal IDs are visible. The signature prevents changes, not disclosure.

Fix: choose a confidential and stable identifier design before exposing new objects. Preserve existing token decoding during migration. Bind contact and calendar IDs to a user or source where needed. Test that raw internal parts are absent from public IDs.

### High: attachment downloads have no byte limit

The bridge reads a full attachment into a PHP string (`gateway/cypht-module/gateway/handler_modules.php:151-159`). The Rust client then reads the full response into memory (`gateway/crates/gateway-cypht/src/lib.rs:244-258`). The domain returns those bytes without a size check (`gateway/crates/gateway-domain/src/lib.rs:293-305`). A large provider attachment can exhaust memory.

Fix: use a bounded streaming path or a strict size limit on both sides. Test an oversized attachment and an upstream response with no length header.

### High: write audit failures are ignored

The domain records writes only after success. Its `audit` helper discards the storage result (`gateway/crates/gateway-domain/src/lib.rs:327-340,731-733`). Failed and uncertain writes have no reliable audit result. This does not meet the v0.5 requirement to record success state for every write.

Fix: define failure and uncertain-send audit semantics. Propagate or separately report an audit storage failure. Never put contact notes, calendar descriptions, feed content, or mail bodies in audit detail.

### Medium: search reports a truncated count as total

The bridge fetches at most `limit` results per account, trims the merged list, and returns `count($result)` as `total` (`gateway/cypht-module/gateway/handler_modules.php:87-108`). It cannot report the actual total or a reliable next page. The REST result can incorrectly appear complete.

Fix: report an unknown total or implement provider-aware pagination. Test more matches than the requested limit and multiple accounts.

### Medium: special folder roles are lost for custom names

The gateway uses `special` only when it is a string (`gateway/cypht-module/gateway/functions.php:54-64`). Cypht's IMAP folder list exposes it as a boolean (`upstream/modules/imap/hm-imap.php:2634-2640`). Name heuristics miss custom or localized Sent, Trash, and Archive folders.

Fix: use Cypht's special-use folder mapping rather than treating this boolean as a role name.

## v0.5 capability comparison

| Feature | Cypht v2.12.2 source | Current gateway | Next work |
|---|---|---|---|
| Contacts | `Hm_Contact_Store` in `modules/contacts/hm-contacts.php`; local, CardDAV, Gmail, and LDAP modules | ID kind and scopes only | Start with local CRUD, search, source checks, user-bound IDs, session persistence, then client surfaces |
| Tags | `Hm_Tags` in `modules/tags/hm-tags.php` stores server, folder, and message links | ID kind and scopes only | Map both tag and message IDs; preserve folder and server context on moves |
| Saved searches | `Hm_Saved_Searches` in `modules/saved_searches/modules.php:487` uses names as keys | ID kind and scopes only | Define stable public identity and rename semantics; preserve advanced-search metadata |
| Calendar | `Hm_Cal_Event` has title, description, date, and repeat interval (`modules/calendar/hm-calendar.php:252-269`) | ID kinds and scopes only | Resolve the gap with requested start/end, location, all-day, and recurrence fields before promising full CRUD |
| Feeds | `Hm_Feed_List` and `feed_url_is_allowed` in `modules/feeds/hm-feed.php` | ID kinds and scopes only | Require explicit HTTP(S), validate host and all resolved IPs, control connection IP and egress, then expose safe items |
| Sieve | Module name is `sievefilters`; `Hm_Sieve_Client_Factory` connects with mailbox credentials | Read scope only | Add status-only Bridge with module/config checks and a bounded, redacted error contract |

Cypht's feed URL check validates DNS before the fetch (`upstream/modules/feeds/hm-feed.php:17-69,171-205`). The fetch performs another resolution. That creates a DNS-rebinding window. It also prepends `http://` to a URL without an HTTP(S) prefix. A gateway endpoint needs stricter URL validation and network egress controls. Cypht disables redirect following in its fetch paths.

The initial groundwork granted unimplemented read scopes to default AI tokens. This is now fixed: `DEFAULT_AI_SCOPES` includes `contacts.read` but not the pending tags, searches, calendar, feeds, or Sieve read scopes.

## Verified alignment

- `process_api_login` and `api_login_key` match Cypht's API login module (`upstream/modules/api_login/modules.php:32-49`).
- The bridge's `get_messages`, `message_action`, `delete_message`, and `store_message` calls use the `Hm_Mailbox` signatures in `upstream/modules/core/hm-mailbox.php`.
- The fourth argument to `Hm_Mailbox::send_message` is the delivery-receipt flag (`upstream/modules/core/hm-mailbox.php:628-640`). Do not confuse it with the lower-level `Hm_SMTP::send_message` signature.

## Repair order and gates

1. Fix the Cypht request header allowlist and add a live ping test.
2. Fix bridge response and session persistence. Test upload followed by send in a second request.
3. Design confidential, user-scoped IDs and migration behavior.
4. Bound attachment downloads and fix audit failure semantics.
5. Implement one optional module at a time through Bridge, domain, REST, OpenAPI, registry, CLI, MCP, UI, and tests.
6. Re-run PHP lint, Cargo build/test/clippy, frontend checks, OpenAPI parsing, and archive extraction before a release tag.

Do not mark v0.5 complete from the ID-kind and scope groundwork alone.

## Live smoke check on 2026-09-23

Target: `http://192.168.8.11:8088/`. This is a Cypht site, not the Rust Gateway API. Login with the provided temporary test account succeeded. The UI identifies the running version as **2.12.0**, while this source audit targets **2.12.2**.

- The home page reports zero IMAP, JMAP, EWS, SMTP, RSS/Atom, and profile entries for this account.
- The Contacts page loads and shows empty local contact groups. The Calendar page loads for September 2026.
- `?page=ajax_gateway_ping` displays Cypht's **Page Not Found** page in the logged-in browser session. The Gateway module is not available on this site.
- `/api/v1/meta/version` displays Cypht HTML rather than the Gateway version JSON at this host and port.

No mail account was configured. No contact, event, feed, or message was created, changed, sent, or deleted. The missing Bridge page prevents an end-to-end test of the gateway code. These results do not validate the static findings against a deployed Gateway Bridge. No credential was written to this report.

## Local repair status on 2026-09-23

- The Bridge now allowlists its private key and upload headers. Its JSON and attachment handlers close the authenticated Cypht session before exiting. These changes passed PHP 8.1 syntax checks and a local Bridge contract test. They have not been deployed to the live Cypht site.
- The Bridge and Rust adapter now reject attachment downloads above 25 MiB. Cypht can still load an unknown-size part into PHP memory before the post-read check.
- Rust builds and workspace tests pass locally with Rust 1.94.1 GNU and a temporary ASCII Cargo target path. A strict Clippy-driver workspace check also passes. The pinned Rust 1.88 toolchain and Linux CI remain unverified.
- Local Contacts now has Bridge, domain, REST, CLI, MCP, UI, OpenAPI, and registry work. Its Cypht v2.12.2 repository CRUD test passes. No live Bridge is installed at `192.168.8.11:8088`, so end-to-end Contacts remains unverified.
- Legacy mail IDs remain signed but readable. New Contact IDs use a keyed digest and owner binding; they do not expose Cypht contact IDs. Tags, saved searches, calendar, feeds, and Sieve status remain incomplete.
- The restored Git checkout still points at v0.3.0, while the v0.4.0 source archive had no Git history. The version gate, a clean release tag, and release archives remain outstanding.
## Origin and mailbox smoke check on 2026-09-23

A fresh authenticated browser tab at `http://localhost:8088/` shows one IMAP account and one SMTP account. The IMAP inbox loads and lists messages. The Cypht native `?page=ajax_hm_folders` page also shows the account. No message body was read, and no mail was sent, moved, or deleted.

The earlier authenticated session at `http://192.168.8.11:8088/` still shows zero accounts after a reload. These origins have separate browser cookies and Cypht sessions. The difference can reflect stale session configuration or a different site mapping; the evidence does not distinguish those causes. The localhost Save page reports no pending changes for its session. A fresh authenticated LAN session would be needed to compare saved configuration.

Both origins show Cypht 2.12.0. On localhost, `?page=ajax_gateway_ping` is still Page Not Found, and `/api/v1/meta/version` renders Cypht HTML rather than Gateway JSON. A configured mailbox makes native Cypht mail reading testable, but does not deploy the Gateway Bridge or REST server. The localhost account has no sending profile, so Gateway send remains untestable without adding one.

The Bridge ping now reports the upstream Cypht version. The Rust adapter rejects releases other than 2.12.2 before serving the mail or Contacts API. This check is not deployed on either observed Cypht origin.
