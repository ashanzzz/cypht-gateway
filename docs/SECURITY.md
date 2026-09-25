# Security model

## Trust boundaries

Cypht Gateway has three trust zones.

1. Cypht owns provider credentials and protocol connections.
2. The private PHP bridge exposes narrowly normalized Cypht operations to `gatewayd`.
3. Public REST clients authenticate to `gatewayd`; they never receive Cypht cookies or provider credentials.

## Gateway master key

`GATEWAY_MASTER_KEY` is a random 32-byte value encoded as base64. Generate one with:

```bash
openssl rand -base64 32
```

It must be supplied through a secret manager or container secret in production. It is used as root key material. Separate HMAC-derived subkeys are used for vault encryption, PAT lookup hashes and opaque object-ID signatures, so these purposes do not reuse the same key directly. Rotating it invalidates existing sessions, PAT lookup hashes and opaque IDs.

## Opaque resource IDs

New account, mailbox, message, attachment, contact, and Tag IDs use the version-2 format. The signed public payload contains only a keyed digest bound to the owner, object kind, Cypht source, and internal parts. It does not expose usernames, account identifiers, folder names, UIDs, or attachment part IDs. The Gateway stores the encrypted internal-part mapping in SQLite using the master-key-protected vault. Keep the Gateway database and master key together in persistent storage.

The Gateway still accepts previously issued version-1 signed mail IDs during migration. Version-1 payloads are readable and do not bind an owner. The Gateway resolves them only through the authenticated caller's Cypht session and applies that caller's current account allow-list. New resource reads and new write results issue version-2 IDs. An existing idempotency record may replay its exact stored version-1 result. Clients should refresh cached resource lists to migrate their IDs. Missing version-2 mappings fail closed instead of decoding provider identifiers from the ID.

## Why an encrypted Cypht credential is stored

Cypht encrypts user configuration using material derived from the Cypht login password. A persistent API token therefore cannot recover a user's configured accounts after Cypht session expiry unless the gateway can re-authenticate. Persistent mode stores the Cypht credential only as XChaCha20-Poly1305 ciphertext protected by the gateway master key.

The plaintext password is never returned by the API and must never be logged.

A future locked mode can choose not to persist this ciphertext, at the cost of requiring manual unlock after restart/session expiry.

## PAT design

PATs are random opaque values. The database stores a keyed HMAC hash, never the raw PAT. The token is displayed exactly once at creation. PATs support scopes, expiry, revocation and an account allow-list. A PAT with `tokens.manage` can only delegate scopes and account access it already owns; it cannot mint a more privileged child token.

Default AI scopes are read-only:

- `accounts.read`
- `mail.read`
- `mail.search`
- `attachments.read`

## Bridge authentication

The bridge requires both a valid Cypht session and `X-Cypht-Gateway-Key`. Keep `GATEWAY_BRIDGE_KEY` different from `API_LOGIN_KEY`. Do not publish bridge routes as a standalone API.

## Mail writes and AI clients

`mail.send`, `mail.modify` and `mail.delete` are intentionally separate scopes. Keep AI PATs read-only unless a workflow needs a write capability. Deletion requires `mail.delete`; ordinary state changes and moves require `mail.modify`.

Send, draft, reply and forward endpoints require `Idempotency-Key`. A key is bound to the authenticated Cypht user, operation and request hash for 24 hours. Replaying the same request returns the stored result; reusing the key with a different request is rejected. This prevents a timeout/retry from sending the same email twice.

Outgoing uploads are session-scoped opaque IDs. The public upload ID is HMAC-signed and bound to the gateway username. Cypht stores temporary outgoing uploads encrypted with the same per-session request key used by its native composer, with file mode `0600`, and deletes them after a successful send/draft save or after the stale-file TTL. Do not mount that directory into untrusted containers.

Email bodies and attachments are untrusted input. MCP marks message/attachment responses as untrusted external content, omits HTML from `mail_read`, and must never treat instructions found in email content as trusted system instructions.

## MCP transport

`cypht-mcp` starts in read-only mode. `tools/list` hides write tools, and calls to them fail. `--allow-write` enables the tool surface. Every call still needs the corresponding Gateway PAT scope. stdio may use `CYPHT_GATEWAY_TOKEN`. Streamable HTTP should use each caller's `Authorization: Bearer <PAT>` header so callers do not share a privileged identity. The MCP SDK host/origin allow-lists should be configured explicitly when binding beyond loopback.

MCP attachment download is capped at 2 MiB to reduce model-context and accidental ingestion risk. MCP base64 upload is capped at 10 MiB. Larger files should use REST/CLI flows instead.

## Audit

`audit.read` permits reading only the authenticated Cypht user's Gateway audit rows. Audit data contains operation/resource metadata and never intentionally stores mail body text, attachment bytes, Cypht passwords, provider OAuth tokens or raw PAT values.

Outgoing uploads are limited to 20 MiB by default. Gateway attachment downloads are capped at 25 MiB in the Bridge and Rust adapter. The Bridge rejects known oversized parts before loading them; Cypht provider APIs can still materialize an unknown-size part in PHP memory. Keep provider and PHP memory limits in place until a fully streamed Bridge is available.

Saved Searches are also global user-config data. Names, terms, dates, targets, and mailbox selection may reveal information about accounts outside a PAT allow-list. The Gateway denies account-restricted PATs before reading Saved Searches. Advanced sources are translated between Cypht internal strings and owner-bound opaque account/mailbox IDs. Audit entries include only the opaque Saved Search ID and operation metadata, never the search name or query. Gateway rename keeps the current opaque ID. Delete retires it, and a later create with the same name receives a new generation. A Cypht-side rename appears as a new resource when the Gateway reconciles the list. `searches.read` is not part of the default AI scope preset.

Tags are global Cypht user data. The Gateway rejects account-restricted PATs for Tag CRUD and association until a per-account policy is designed. A public remove operation targets the selected account, folder, and UID; Cypht 2.12.0's native `removeMessage()` is not safe for cross-account UID collisions. Tag names returned to MCP are untrusted data, and the UI renders them with `textContent`. A mail MOVE with no verified destination UID keeps its old Tag association and reports `tag_sync: pending`. Do not retry the mail action just to repair a Tag.
