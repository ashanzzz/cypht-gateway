# Cypht Gateway capability parity

This file is the explicit completeness ledger for AI and human maintainers. A row marked `complete` means the Gateway has a stable public representation of that Cypht capability. It does not mean every provider behaves identically; provider-specific limits remain governed by Cypht and the upstream mail service.

| Cypht capability | REST | CLI | MCP | Management UI | v0.4 status |
|---|---|---|---|---|---|
| Login/session | yes | token-based client use | no direct login tool | yes | complete |
| Personal API tokens/scopes | yes | yes | no token-management tools | yes | complete |
| Account list | yes | yes | yes | yes | complete |
| Mailbox/folder list | yes | yes | yes | partial | complete API surface |
| Unified message list | yes | yes | yes | yes | complete |
| Search | yes | yes | yes | yes | complete |
| Read text message | yes | yes | yes | yes | complete |
| Attachment metadata/download | yes | yes | yes | yes | complete |
| Attachment upload | yes | yes | yes | yes | complete |
| Send | yes | yes | yes | yes | complete |
| Reply/reply-all | yes | yes | yes | yes | complete |
| Forward | yes | yes | yes | yes | complete |
| Draft/create scheduled message | yes | yes | yes | yes | complete |
| Seen/flagged state | yes | yes | yes | yes | complete |
| Move/archive/delete | yes | yes | yes | yes | complete |
| Audit log | yes | yes | yes | yes | complete |
| MCP stdio | n/a | n/a | yes | setup help | complete |
| MCP Streamable HTTP | n/a | n/a | yes | setup help | complete |
| Contact sources / CardDAV / local contacts | local routes implemented; custom file adapter tested; live writes unverified | CLI mapping implemented; live Bridge writes unverified | MCP mapping implemented; live Bridge writes unverified | UI mapping implemented; live Bridge writes unverified | local file adapter tests pass; live Unraid readback and write conflicts unverified; DB storage fails closed; CardDAV, Google, LDAP deferred |
| Calendar | synthetic user calendar, event list/create/delete | CLI list/events/create/delete | read/list/create/delete tools, writes opt-in | list/create/delete | local Cypht 2.12.0 adapter and source tests pass; update, all-day, end time, location, RRULE, and live Unraid tests remain deferred. |
| Tags | CRUD and association routes implemented; custom file adapter tested; live writes unverified | local mappings implemented; live Bridge writes unverified | MCP tools implemented; live Bridge writes unverified | UI implemented; live Bridge writes unverified | local file adapter tests pass; live Unraid readback and write conflicts unverified; DB storage fails closed; MOVE without COPYUID stays pending |
| Saved searches | CRUD metadata | CRUD metadata | read/write tools, writes opt-in | list/create/update/delete | local implementation and source tests pass; live Unraid Bridge, REST, CLI, MCP, and UI verification pending. Stable IDs survive successful Gateway rename and retire on delete or detected external rename. Advanced sources use opaque account/mailbox IDs. Account-restricted PATs are denied and searches.read is not a default AI scope. |
| Sieve filters | redacted status | sieve-status | read-only status tool | status panel | redacted status is implemented locally. Script and filter writes, ManageSieve probing, and live Unraid tests are deferred. |
| Feeds | read-only subscription metadata | feeds, feed-read | feeds_list, feeds_get | subscription list | read-only metadata implemented; article fetching and feed writes deferred for SSRF protection |
| Profiles CRUD | list only | list only | list only | selection only | deferred to v0.5+ |
| Account/server configuration CRUD | no | no | no | no | deferred to v0.5+ |
| General Cypht settings/themes/shortcuts | no | no | no | no | deferred to v0.5+ |
| PGP/key management | no | no | no | no | deferred to v0.5+ |
| Password/2FA/admin account management | no | no | intentionally no | no | intentionally outside AI tool surface for now |

The complete Cypht 2.12.0 source-module mapping is recorded in [`docs/CYPHT_MODULE_MATRIX.md`](CYPHT_MODULE_MATRIX.md). It distinguishes source-level mapping from runtime module enablement and verified deployment behavior. Current command and runtime evidence is recorded in [`docs/VALIDATION.md`](VALIDATION.md).

## v0.4 completeness definition

v0.4 is complete for the **mail automation surface** promised by the v0.1-v0.4 roadmap: authenticated reading and writing, attachments, safe retries, auditability, REST, CLI, management UI, and MCP transports. It is **not** full parity with every Cypht frontend module.

When adding a capability, update this table in the same commit as the domain/API implementation and its client surfaces. Do not mark a capability complete based only on a route stub or registry entry.

Saved Search ID, advanced source, and scope semantics are defined in [`docs/SAVED_SEARCHES.md`](SAVED_SEARCHES.md).

## v0.5 work in progress

New account, mailbox, message, and attachment responses now use version-2 owner-, type-, and source-bound IDs. Gateway SQLite stores encrypted internal-part mappings. The version-1 decoder remains for existing mail IDs, and PAT account allow-lists accept old account IDs during migration. Local Cargo tests passed on Rust 1.94.1 GNU. The pinned Rust 1.88 CI toolchain remains unverified. See `docs/VALIDATION.md`.

Local Contacts and Tags have code across Bridge, Domain, REST, CLI, MCP, UI, audit, contracts, and local tests. The custom Cypht file-settings adapter passed encrypted readback, stale-writer, and wrong-key tests. Stock and database-backed settings still fail closed. The live Unraid Bridge and simultaneous native-UI/Gateway writes remain unverified. Treat lost write responses as uncertain. Do not use real-data writes before live verification. Saved Searches now has local CRUD metadata across all client surfaces. Live Bridge verification remains pending. Calendar has a limited local list/create/delete adapter. Sieve has redacted local status across client surfaces. Feeds remain incomplete. Do not mark v0.5 complete until all capability surfaces and release gates pass.
