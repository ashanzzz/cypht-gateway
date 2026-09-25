use super::*;
use gateway_auth::{AuthKind, AuthService, Credential, CyphtSession, Vault};

fn principal(username: &str) -> Principal {
    Principal {
        username: username.into(),
        auth_kind: AuthKind::Session,
        auth_id: None,
        scopes: vec!["*".into()],
        account_allowlist: Vec::new(),
        credential: Credential {
            username: username.into(),
            password: "test-only-password".into(),
        },
        cypht_session: CyphtSession {
            hm_id: "test-id".into(),
            hm_session: "test-session".into(),
        },
    }
}

#[test]
fn saved_search_identity_survives_rename_and_retires_on_delete() {
    let store = Store::open_memory().expect("store");
    let vault = Vault::from_base64("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=").expect("vault");
    let auth = AuthService::new(store.clone(), vault, 3600);
    let ids = ObjectIdCodec::new([13_u8; 32]).expect("id codec");
    let alice = principal("alice");

    let id =
        ensure_saved_search_id(&ids, &auth, &store, &alice, "Inbox query").expect("initial ID");
    assert_eq!(
        ensure_saved_search_id(&ids, &auth, &store, &alice, "Inbox query").expect("stable ID"),
        id
    );
    retire_saved_search_id(&ids, &auth, &store, &alice, &id).expect("prepare rename");
    complete_saved_search_rename(&ids, &auth, &store, &alice, &id, "Unread mail")
        .expect("complete rename mapping");
    assert_eq!(
        saved_search_id_name(&ids, &auth, &store, &alice, &id)
            .expect("resolve")
            .as_deref(),
        Some("Unread mail")
    );
    assert_eq!(
        ensure_saved_search_id(&ids, &auth, &store, &alice, "Unread mail")
            .expect("renamed identity"),
        id
    );

    retire_saved_search_id(&ids, &auth, &store, &alice, &id).expect("retire");
    assert!(saved_search_id_name(&ids, &auth, &store, &alice, &id).is_ok_and(|name| name.is_none()));
    let recreated =
        ensure_saved_search_id(&ids, &auth, &store, &alice, "Unread mail").expect("new generation");
    assert_ne!(recreated, id);
    let external_id = ensure_saved_search_id(&ids, &auth, &store, &alice, "Native query")
        .expect("external search identity");
    retire_missing_saved_search_ids(
        &ids,
        &auth,
        &store,
        &alice,
        &std::collections::HashSet::new(),
    )
    .expect("retire missing native search");
    let after_external_delete = ensure_saved_search_id(&ids, &auth, &store, &alice, "Native query")
        .expect("new identity after external deletion");
    assert_ne!(after_external_delete, external_id);
    let bob = principal("bob");
    let bob_id =
        ensure_saved_search_id(&ids, &auth, &store, &bob, "Unread mail").expect("owner-bound ID");
    assert_ne!(bob_id, recreated);
}
