use super::{router, ApiState};
use axum::{
    body::Body,
    http::{header::CONTENT_TYPE, Method, Request, StatusCode},
};
use gateway_auth::{AuthService, Vault};
use gateway_core::{BuildInfo, ObjectIdCodec};
use gateway_cypht::{CyphtClient, CyphtConfig};
use gateway_domain::GatewayService;
use gateway_storage::Store;
use tower::ServiceExt;

fn test_state() -> ApiState {
    let store = Store::open_memory().unwrap();
    let vault = Vault::from_base64("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=").unwrap();
    let auth = AuthService::new(store.clone(), vault, 3600);
    let cypht = CyphtClient::new(CyphtConfig {
        base_url: "http://127.0.0.1/".parse().unwrap(),
        api_login_key: "unused".into(),
        bridge_key: "unused".into(),
    })
    .unwrap();
    let service = GatewayService::new(auth, cypht, ObjectIdCodec::new([7_u8; 32]).unwrap(), store);
    ApiState {
        service,
        build: BuildInfo::current(),
    }
}

#[tokio::test]
async fn tags_routes_require_authentication() {
    let app = router(test_state());
    let cases = [
        (Method::GET, "/api/v1/tags", ""),
        (Method::POST, "/api/v1/tags", r#"{"name":"Work"}"#),
        (Method::PATCH, "/api/v1/tags/id", r#"{"name":"Work"}"#),
        (Method::DELETE, "/api/v1/tags/id?confirm=true", ""),
        (Method::POST, "/api/v1/messages/msg/tags/tag", ""),
        (
            Method::DELETE,
            "/api/v1/messages/msg/tags/tag?confirm=true",
            "",
        ),
    ];
    for (method, uri, body) in cases {
        let mut request = Request::builder().method(method.clone()).uri(uri);
        if !body.is_empty() {
            request = request.header(CONTENT_TYPE, "application/json");
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::from(body)).unwrap())
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "{method} {uri}"
        );
    }
}
#[tokio::test]
async fn saved_search_routes_require_authentication() {
    let app = router(test_state());
    let cases = [
        (Method::GET, "/api/v1/saved-searches", ""),
        (
            Method::POST,
            "/api/v1/saved-searches",
            r#"{"name":"Work","type":"simple","query":"invoice"}"#,
        ),
        (Method::GET, "/api/v1/saved-searches/id", ""),
        (
            Method::PATCH,
            "/api/v1/saved-searches/id",
            r#"{"name":"Renamed"}"#,
        ),
        (Method::DELETE, "/api/v1/saved-searches/id?confirm=true", ""),
    ];
    for (method, uri, body) in cases {
        let mut request = Request::builder().method(method.clone()).uri(uri);
        if !body.is_empty() {
            request = request.header(CONTENT_TYPE, "application/json");
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::from(body)).unwrap())
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "{method} {uri}"
        );
    }
}
#[tokio::test]
async fn calendar_routes_require_authentication() {
    let app = router(test_state());
    let cases = [
        (Method::GET, "/api/v1/calendars", ""),
        (
            Method::GET,
            "/api/v1/calendars/id/events?start=2026-09-01T00:00:00Z&end=2026-10-01T00:00:00Z",
            "",
        ),
        (
            Method::POST,
            "/api/v1/calendars/id/events",
            r#"{"title":"Test","starts_at":"2026-09-24T09:00:00Z"}"#,
        ),
        (
            Method::DELETE,
            "/api/v1/calendars/id/events/event?confirm=true",
            "",
        ),
    ];
    for (method, uri, body) in cases {
        let mut request = Request::builder().method(method.clone()).uri(uri);
        if !body.is_empty() {
            request = request.header(CONTENT_TYPE, "application/json");
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::from(body)).unwrap())
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "{method} {uri}"
        );
    }
}
#[tokio::test]
async fn sieve_routes_require_authentication() {
    let app = router(test_state());
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/sieve/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn feeds_routes_require_authentication() {
    let app = router(test_state());
    let cases = [
        (Method::GET, "/api/v1/feeds"),
        (Method::GET, "/api/v1/feeds/id"),
    ];
    for (method, uri) in cases {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method.clone())
                    .uri(uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "{method} {uri}"
        );
    }
}
