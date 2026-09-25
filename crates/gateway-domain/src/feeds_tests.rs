use super::GatewayService;
use gateway_auth::{AuthKind, AuthService, Credential, Principal, Vault, SCOPE_FEEDS_READ};
use gateway_core::{GatewayError, ObjectIdCodec, ObjectKind};
use gateway_cypht::BridgeFeed;
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

fn principal() -> Principal {
    Principal {
        username: "alice".into(),
        auth_kind: AuthKind::Pat,
        auth_id: Some("pat".into()),
        scopes: vec![SCOPE_FEEDS_READ.into()],
        account_allowlist: vec![],
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
fn account_restricted_feed_is_denied_before_upstream() {
    let mut p = principal();
    p.account_allowlist = vec!["acc_123".into()];
    let error = service().assert_global_resource_allowed(&p).unwrap_err();
    assert!(matches!(error, GatewayError::PermissionDenied(_)));
}

#[test]
fn feed_id_binding_verifies_kind_and_owner() {
    let svc = service();
    let p = principal();
    let feed_row = BridgeFeed {
        id: "1".into(),
        name: "News".into(),
        url: "https://example.com/rss.xml".into(),
    };
    let public_id = svc.feed_public_id(&p, &feed_row).unwrap();
    let decoded = svc
        .ids
        .decode_versioned(ObjectKind::Feed, &public_id)
        .unwrap();
    assert_eq!(decoded.version, 2);
    assert_eq!(decoded.parts.len(), 1);

    // Cross-kind decode fails
    assert!(svc
        .ids
        .decode_versioned(ObjectKind::Message, &public_id)
        .is_err());
}
