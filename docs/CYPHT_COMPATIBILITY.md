# Cypht compatibility

## v0.2 baseline

The default bridge image is pinned to Cypht `v2.12.2`. The bridge contract was reviewed against that tag's `api_login`, `Hm_IMAP_List`, `Hm_Mailbox` and AJAX-page APIs. Runtime integration still belongs in CI because this build environment does not contain a live Cypht instance.


The bridge targets the current Cypht module architecture where:

- `api_login` can create a Cypht session from username/password plus `API_LOGIN_KEY`.
- `setup_base_ajax_page()` loads normal authenticated session context.
- `Hm_IMAP_List` initializes configured IMAP/JMAP/EWS accounts.
- `Hm_Mailbox` provides common folder, list, message and search operations.

The bridge deliberately calls these higher-level abstractions rather than provider-specific classes.

## Compatibility policy before 1.0

`0.x` releases may adjust the bridge for upstream Cypht changes without preserving the internal bridge RPC. Public REST `/api/v1` should remain stable whenever practical.

CI should eventually test both the pinned supported Cypht release and upstream `master`. A failure against upstream `master` is an early-warning compatibility regression, not automatically a public API break.
