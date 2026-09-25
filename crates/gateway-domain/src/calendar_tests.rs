use super::{parse_timestamp, repeat_from_cypht, GatewayService};
use gateway_auth::{AuthKind, AuthService, Credential, Principal, Vault, SCOPE_CALENDAR_READ};
use gateway_core::{CalendarRepeatInterval, GatewayError, ObjectIdCodec};
use gateway_storage::Store;

fn service() -> GatewayService {
    let store = Store::open_memory().expect("store");
    let vault = Vault::from_base64("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=").expect("vault");
    let auth = AuthService::new(store.clone(), vault, 3600);
    let cypht = gateway_cypht::CyphtClient::new(gateway_cypht::CyphtConfig {
        base_url: "http://127.0.0.1:1/".parse().expect("url"),
        api_login_key: "unused".into(),
        bridge_key: "unused".into(),
    })
    .expect("client");
    GatewayService::new(
        auth,
        cypht,
        ObjectIdCodec::new([5_u8; 32]).expect("ids"),
        store,
    )
}

fn restricted_principal() -> Principal {
    Principal {
        username: "alice".into(),
        auth_kind: AuthKind::Pat,
        auth_id: Some("pat".into()),
        scopes: vec![SCOPE_CALENDAR_READ.into()],
        account_allowlist: vec!["account".into()],
        credential: Credential {
            username: "alice".into(),
            password: "unused".into(),
        },
        cypht_session: gateway_auth::CyphtSession {
            hm_id: "id".into(),
            hm_session: "session".into(),
        },
    }
}

#[test]
fn calendar_timestamp_requires_offset_and_repeat_is_limited() {
    assert_eq!(
        parse_timestamp("2026-09-24T09:00:00-07:00", "start").unwrap(),
        1790265600
    );
    assert!(parse_timestamp("2026-09-24T09:00:00", "start").is_err());
    assert_eq!(
        repeat_from_cypht("week").unwrap(),
        CalendarRepeatInterval::Week
    );
    assert!(matches!(
        repeat_from_cypht("rrule"),
        Err(GatewayError::CapabilityUnavailable(_))
    ));
}

#[test]
fn account_restricted_calendar_is_denied_before_upstream() {
    let error = service()
        .assert_global_resource_allowed(&restricted_principal())
        .unwrap_err();
    assert!(matches!(error, GatewayError::PermissionDenied(_)));
}
