# Cypht Gateway private bridge

This module is the only Cypht-specific adapter in Cypht Gateway. It is deliberately **not** a public REST API.

## Security contract

A bridge request must satisfy both conditions:

1. It has a valid authenticated Cypht session (`hm_id` + `hm_session`).
2. It presents `X-Cypht-Gateway-Key`, equal to the Cypht environment variable `GATEWAY_BRIDGE_KEY`.

The bridge never returns IMAP/SMTP passwords, OAuth tokens, or the full server repository. Public clients should only call the Rust gateway.

## Install

Copy this directory to `modules/gateway` in the Cypht installation. Keep Cypht's normal `core,imap,smtp,profiles` modules enabled and add both `api_login` and `gateway` to `CYPHT_MODULES`. Enable `scheduled_sends` when the API should support future delivery times.

Set two independent random secrets in Cypht:

```env
API_LOGIN_KEY=<random secret shared only with gatewayd>
GATEWAY_BRIDGE_KEY=<different random secret shared only with gatewayd>
```

The Rust gateway needs matching `CYPHT_API_LOGIN_KEY` and `CYPHT_BRIDGE_KEY` values.

## v0.3 write operations

The bridge now exposes private write handlers for sending, drafts, scheduled sends, message flags, moves, archive and delete. They still require both the Cypht session and bridge key.

Temporary outgoing attachments are encrypted with Cypht's per-session request key, written under Cypht's configured attachment directory with mode `0600`, referenced only by session-scoped upload IDs, and removed after a successful send/draft save or after the stale-file TTL. Configure the maximum raw upload size with:

```env
GATEWAY_MAX_UPLOAD_BYTES=20971520
```

Do not expose any `ajax_gateway_*` page directly as a public API.
