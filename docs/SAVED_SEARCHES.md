# Saved Searches

This document defines the Gateway v0.5 Saved Searches contract for Cypht 2.12.0.

## Public API

The canonical surface is REST v1:

- `GET /api/v1/saved-searches` lists simple and advanced metadata.
- `POST /api/v1/saved-searches` creates one saved search.
- `GET /api/v1/saved-searches/{id}` reads one saved search.
- `PATCH /api/v1/saved-searches/{id}` updates fields or renames it.
- `DELETE /api/v1/saved-searches/{id}?confirm=true` deletes it.

The CLI, MCP server, and management UI call these endpoints. They do not call Bridge pages directly.

## IDs and rename rules

Cypht 2.12.0 keys saved searches by name. The Gateway assigns a random generation ID and stores its name mapping as encrypted user-bound Gateway data. The public ID is a keyed digest bound to owner, resource type, and source.

A successful Gateway rename keeps the public ID. A delete retires that ID. Creating another search with the same name creates a new ID. The Gateway rejects a rename when another search already has the destination name. It does not call Cypht's overwrite-on-rename method.

A native Cypht rename appears as a new Gateway resource when list reconciliation sees that the old name disappeared. The new search gets a new ID. Native delete followed by recreation with the same name between reconciliation requests is indistinguishable in Cypht's name-keyed store. Do not use this behavior as an identity guarantee.

## Simple and advanced data

Simple searches expose `query`, `since`, and `field`. The Gateway accepts only Cypht's supported search fields and time presets.

Advanced searches preserve Cypht 2.12.0 `terms`, `targets`, `times`, and `other` metadata. The Gateway replaces each Cypht source string with an opaque `account_id` and optional `mailbox_id`. It never returns internal Cypht account IDs, folder names encoded in source strings, or raw source strings.

Names, terms, targets, and mailbox choices are user data. The audit log stores only the operation and opaque search ID. MCP labels returned metadata as untrusted user data and excludes write tools unless runtime write mode is enabled.

## Permissions and persistence

Saved Searches are user-wide Cypht settings. The Gateway rejects account-restricted PATs before it reads or writes them. `searches.read` and `searches.write` are explicit scopes. The default AI scope preset grants neither scope.

Writes require a verified durable Cypht user-config adapter. The Bridge takes a fresh snapshot under a per-user lock, rejects stale state, commits the Cypht repository change, and verifies a fresh-load readback. Database-backed user-config mode does not advertise this adapter and remains unavailable. A lost Bridge response is an uncertain result. Clients must refresh the collection and must not blindly replay writes.

## Verification status

Cypht 2.12.0 repository and Bridge source tests cover simple and advanced metadata, rename collisions, confirmation, module-unavailable errors, opaque source conversion, owner binding, and ID retirement. Local Rust, PHP, and contract checks pass. Live Unraid Bridge and API tests remain unverified because the private Bridge and matching secrets are not installed on the current Cypht container.
