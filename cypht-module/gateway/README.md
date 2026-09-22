# Cypht Gateway private bridge

This module is the only Cypht-specific adapter in Cypht Gateway. It is deliberately **not** a public REST API.

## Security contract

A bridge request must satisfy both conditions:

1. It has a valid authenticated Cypht session (`hm_id` + `hm_session`).
2. It presents `X-Cypht-Gateway-Key`, equal to the Cypht environment variable `GATEWAY_BRIDGE_KEY`.

The bridge never returns IMAP/SMTP passwords, OAuth tokens, or the full server repository. Public clients should only call the Rust gateway.

## Install

Copy this directory to `modules/gateway` in the Cypht installation, then enable both `api_login` and `gateway` in `CYPHT_MODULES`.

Set two independent random secrets in Cypht:

```env
API_LOGIN_KEY=<random secret shared only with gatewayd>
GATEWAY_BRIDGE_KEY=<different random secret shared only with gatewayd>
```

The Rust gateway needs matching `CYPHT_API_LOGIN_KEY` and `CYPHT_BRIDGE_KEY` values.
