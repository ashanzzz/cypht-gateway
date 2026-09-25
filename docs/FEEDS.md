# Feeds capability

The Gateway exposes read-only RSS/Atom subscription metadata from Cypht 2.12.0 `feeds` module.

- `GET /api/v1/feeds` (list subscriptions)
- `GET /api/v1/feeds/{id}` (read subscription metadata)
- Scope: `feeds.read`
- Account-restricted PATs are denied access because feeds are user-wide resources, not scoped to individual mailboxes.
- The default AI scope preset does not include `feeds.read`.

## Security invariants

1. **Read-only metadata**: v0.5 only exposes subscription metadata (opaque ID, name, feed URL) already saved in Cypht. It does not initiate outbound HTTP requests or remote fetching.
2. **SSRF protection**: Remote article fetching and subscription creation remain deferred until strict SSRF, egress IP filtering, and DNS rebinding protections are verified.
3. **Opaque IDs**: Public feed IDs use signed versioned HMAC digests bound to user and kind (`ObjectKind::Feed`). Raw Cypht internal array indices are never exposed.
4. **Audit**: Access is audited with username, operation (`feeds.list`, `feeds.read`), resource ID, and timestamp. Feed contents and URLs are not audited.