use axum::{
    extract::{Path, Query, State},
    http::{
        header::{AUTHORIZATION, CACHE_CONTROL},
        HeaderMap, HeaderValue, StatusCode,
    },
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use gateway_auth::{DEFAULT_AI_SCOPES, Principal};
use gateway_core::{ApiErrorBody, ApiErrorDetail, BuildInfo, CreateTokenRequest, GatewayError, LoginRequest, MeResponse, SearchRequest};
use gateway_domain::GatewayService;
use serde::Deserialize;
use std::sync::Arc;
use tower_http::{
    limit::RequestBodyLimitLayer,
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};

#[derive(Clone)]
pub struct ApiState {
    pub service: GatewayService,
    pub build: BuildInfo,
}

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/", get(ui))
        .route("/healthz", get(health))
        .route("/api/v1/meta/version", get(version))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/me", get(me))
        .route("/api/v1/tokens", get(tokens_list).post(tokens_create))
        .route("/api/v1/tokens/{id}", delete(tokens_revoke))
        .route("/api/v1/accounts", get(accounts))
        .route("/api/v1/accounts/{id}/mailboxes", get(mailboxes))
        .route("/api/v1/messages/search", post(search))
        .route("/api/v1/messages", get(messages))
        .route("/api/v1/messages/{id}", get(message))
        .layer(SetResponseHeaderLayer::if_not_present(
            CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        .layer(RequestBodyLimitLayer::new(2 * 1024 * 1024))
        .layer(TraceLayer::new_for_http())
        .with_state(Arc::new(state))
}

async fn ui() -> Html<&'static str> {
    Html(include_str!("../../../frontend/static/index.html"))
}

async fn health(State(state): State<Arc<ApiState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"status":"ok","version":state.build.version}))
}

async fn version(State(state): State<Arc<ApiState>>) -> Json<BuildInfo> {
    Json(state.build.clone())
}

async fn login(State(state): State<Arc<ApiState>>, Json(request): Json<LoginRequest>) -> ApiResult<impl IntoResponse> {
    Ok((StatusCode::OK, Json(state.service.login(request.username, request.password).await?)))
}

async fn logout(State(state): State<Arc<ApiState>>, headers: HeaderMap) -> ApiResult<StatusCode> {
    let bearer = bearer(&headers)?;
    let principal = state.service.authenticate(bearer)?;
    state.service.logout(bearer, &principal)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn me(State(state): State<Arc<ApiState>>, headers: HeaderMap) -> ApiResult<Json<MeResponse>> {
    let principal = principal(&state, &headers)?;
    Ok(Json(MeResponse {
        username: principal.username,
        auth_kind: match principal.auth_kind { gateway_auth::AuthKind::Session => "session", gateway_auth::AuthKind::Pat => "pat" }.into(),
        scopes: principal.scopes,
    }))
}

async fn tokens_list(State(state): State<Arc<ApiState>>, headers: HeaderMap) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(state.service.list_pats(&principal)?))
}

async fn tokens_create(State(state): State<Arc<ApiState>>, headers: HeaderMap, Json(mut request): Json<CreateTokenRequest>) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    if request.scopes.is_empty() {
        request.scopes = DEFAULT_AI_SCOPES.iter().map(|s| (*s).to_string()).collect();
    }
    let result = state.service.create_pat(&principal, &request.name, request.scopes, request.account_allowlist, request.expires_in_days)?;
    Ok((StatusCode::CREATED, Json(result)))
}

async fn tokens_revoke(State(state): State<Arc<ApiState>>, headers: HeaderMap, Path(id): Path<String>) -> ApiResult<StatusCode> {
    let principal = principal(&state, &headers)?;
    state.service.revoke_pat(&principal, &id)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn accounts(State(state): State<Arc<ApiState>>, headers: HeaderMap) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(state.service.accounts(&principal).await?))
}

async fn mailboxes(State(state): State<Arc<ApiState>>, headers: HeaderMap, Path(id): Path<String>) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(state.service.mailboxes(&principal, &id).await?))
}

#[derive(Debug, Deserialize)]
struct MessagesQuery {
    account_id: Option<String>,
    mailbox_id: Option<String>,
    offset: Option<u32>,
    limit: Option<u32>,
}

async fn messages(State(state): State<Arc<ApiState>>, headers: HeaderMap, Query(query): Query<MessagesQuery>) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(state.service.messages(
        &principal,
        query.account_id.as_deref(),
        query.mailbox_id.as_deref(),
        query.offset.unwrap_or(0),
        query.limit.unwrap_or(50),
    ).await?))
}

async fn search(State(state): State<Arc<ApiState>>, headers: HeaderMap, Json(request): Json<SearchRequest>) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(state.service.search(&principal, request).await?))
}

async fn message(State(state): State<Arc<ApiState>>, headers: HeaderMap, Path(id): Path<String>) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(state.service.message(&principal, &id).await?))
}

fn principal(state: &ApiState, headers: &HeaderMap) -> ApiResult<Principal> {
    Ok(state.service.authenticate(bearer(headers)?)?)
}

fn bearer(headers: &HeaderMap) -> ApiResult<&str> {
    let raw = headers.get(AUTHORIZATION).and_then(|v| v.to_str().ok()).ok_or(GatewayError::Authentication)?;
    raw.strip_prefix("Bearer ").filter(|v| !v.is_empty()).ok_or(GatewayError::Authentication).map_err(ApiError)
}

struct ApiError(GatewayError);
type ApiResult<T> = Result<T, ApiError>;

impl From<GatewayError> for ApiError {
    fn from(value: GatewayError) -> Self { Self(value) }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self.0 {
            GatewayError::Authentication => StatusCode::UNAUTHORIZED,
            GatewayError::PermissionDenied(_) => StatusCode::FORBIDDEN,
            GatewayError::NotFound(_) => StatusCode::NOT_FOUND,
            GatewayError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            GatewayError::Upstream(_) => StatusCode::BAD_GATEWAY,
            GatewayError::Configuration(_) | GatewayError::Storage(_) | GatewayError::Crypto | GatewayError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = ApiErrorBody {
            error: ApiErrorDetail {
                code: self.0.code().into(),
                message: self.0.to_string(),
                request_id: None,
            }
        };
        (status, Json(body)).into_response()
    }
}
