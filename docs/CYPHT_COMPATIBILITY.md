# Cypht compatibility

## v0.3 baseline

The default bridge image is pinned to Cypht `v2.12.2`. The bridge contract is reviewed against that tag's `api_login`, `Hm_IMAP_List`, `Hm_Mailbox`, `Hm_SMTP_List`, `Hm_Profiles`, `Hm_MIME_Msg`, scheduled-send and AJAX-page APIs.

The bridge targets the current Cypht module architecture where:

- `api_login` creates a Cypht session from username/password plus `API_LOGIN_KEY`.
- `setup_base_ajax_page()` loads normal authenticated session context.
- `Hm_IMAP_List` initializes configured IMAP/JMAP/EWS accounts.
- `Hm_Mailbox` provides common folder, message and mutation operations.
- `Hm_SMTP_List`, `Hm_Profiles` and `Hm_MIME_Msg` provide the existing send path.
- Cypht's Scheduled mailbox and scheduler remain responsible for delivery of scheduled messages created by the gateway.

The bridge deliberately calls these higher-level abstractions rather than provider-specific protocol classes.

## Compatibility policy before 1.0

`0.x` releases may adjust the private bridge RPC for upstream Cypht changes without preserving that internal RPC. Public REST `/api/v1` should remain stable whenever practical.

The gateway and bridge perform an exact product-version handshake. A mismatched bridge is rejected rather than silently running against an unknown internal contract.

Runtime integration against a live provider belongs in deployment/CI environments with test mailboxes. This artifact environment does not contain live Cypht/Gmail/QQ credentials.
