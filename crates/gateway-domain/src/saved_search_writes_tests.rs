use crate::GatewayService;
use gateway_auth::{
    AuthKind, AuthService, Credential, CyphtSession, Principal, Vault, SCOPE_SEARCHES_READ,
    SCOPE_SEARCHES_WRITE,
};
use gateway_core::{GatewayError, ObjectIdCodec};
use gateway_storage::Store;

fn test_service() -> GatewayService {
    let store = Store::open_memory().expect("store");
    let vault = Vault::from_base64("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=").expect("vault");
    let auth = AuthService::new(store.clone(), vault, 3600);
    let cypht = gateway_cypht::CyphtClient::new(gateway_cypht::CyphtConfig {
        base_url: "http://127.0.0.1:1/".parse().expect("url"),
        api_login_key: "unused".into(),
        bridge_key: "unused".into(),
    })
    .expect("Cypht client");
    GatewayService::new(
        auth,
        cypht,
        ObjectIdCodec::new([3_u8; 32]).expect("id codec"),
        store,
    )
}

#[test]
fn account_restricted_pats_cannot_access_user_wide_saved_searches() {
    let service = test_service();
    let principal = Principal {
        username: "alice".into(),
        auth_kind: AuthKind::Pat,
        auth_id: Some("pat-1".into()),
        scopes: vec![SCOPE_SEARCHES_READ.into(), SCOPE_SEARCHES_WRITE.into()],
        account_allowlist: vec!["opaque-account".into()],
        credential: Credential {
            username: "alice".into(),
            password: "unused".into(),
        },
        cypht_session: CyphtSession {
            hm_id: "unused".into(),
            hm_session: "unused".into(),
        },
    };
    assert!(principal.requires(SCOPE_SEARCHES_READ).is_ok());
    assert!(principal.requires(SCOPE_SEARCHES_WRITE).is_ok());
    assert!(matches!(
        service.assert_global_resource_allowed(&principal),
        Err(GatewayError::PermissionDenied(_))
    ));
}
