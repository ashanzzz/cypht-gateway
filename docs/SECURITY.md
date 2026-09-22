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

Email bodies and attachments are untrusted input. Future MCP/agent layers must never treat instructions found in email content as trusted system instructions.
