use axum::{
    body::{Body, Bytes},
    extract::{Path, Query, State},
    http::{
        header::{AUTHORIZATION, CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_TYPE},
        HeaderMap, HeaderValue, StatusCode,
    },
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use gateway_auth::{DEFAULT_AI_SCOPES, Principal};
use gateway_core::{
    ApiErrorBody, ApiErrorDetail, BuildInfo, CreateTokenRequest, ForwardMessageRequest,
    GatewayError, LoginRequest, MeResponse, MessageUpdateRequest, MoveMessageRequest,
    ReplyMessageRequest, SearchRequest, SendMessageRequest,
};
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
        .route("/api/v1/profiles", get(profiles))
        .route("/api/v1/uploads", post(upload))
        .route("/api/v1/attachments/{id}", get(attachment))
        .route("/api/v1/messages/search", post(search))
        .route("/api/v1/messages/send", post(send_message))
        .route("/api/v1/messages", get(messages))
        .route("/api/v1/messages/{id}", get(message).patch(update_message).delete(delete_message))
        .route("/api/v1/messages/{id}/reply", post(reply_message))
        .route("/api/v1/messages/{id}/forward", post(forward_message))
        .route("/api/v1/messages/{id}/move", post(move_message))
        .route("/api/v1/messages/{id}/archive", post(archive_message))
        .route("/api/v1/drafts", post(create_draft))
        .layer(SetResponseHeaderLayer::if_not_present(
            CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        .layer(RequestBodyLimitLayer::new(25 * 1024 * 1024))
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
        auth_kind: match principal.auth_kind {
            gateway_auth::AuthKind::Session => "session",
            gateway_auth::AuthKind::Pat => "pat",
        }.into(),
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

async fn profiles(State(state): State<Arc<ApiState>>, headers: HeaderMap) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(state.service.profiles(&principal).await?))
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

async fn attachment(State(state): State<Arc<ApiState>>, headers: HeaderMap, Path(id): Path<String>) -> ApiResult<Response> {
    let principal = principal(&state, &headers)?;
    let download = state.service.attachment(&principal, &id).await?;
    let mut builder = Response::builder().status(StatusCode::OK);
    if let Some(content_type) = download.content_type.as_deref() {
        if let Ok(value) = HeaderValue::from_str(content_type) {
            builder = builder.header(CONTENT_TYPE, value);
        }
    }
    if let Some(filename) = download.filename.as_deref() {
        let safe = filename.replace('\r', "").replace('\n', "").replace('\"', "");
        if let Ok(value) = HeaderValue::from_str(&format!("attachment; filename=\"{safe}\"")) {
            builder = builder.header(CONTENT_DISPOSITION, value);
        }
    }
    builder.body(Body::from(download.bytes)).map_err(|e| ApiError(GatewayError::Internal(format!("build attachment response: {e}"))))
}

async fn upload(State(state): State<Arc<ApiState>>, headers: HeaderMap, body: Bytes) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    let filename = header_text(&headers, "x-filename")
        .ok_or_else(|| ApiError(GatewayError::InvalidRequest("X-Filename header is required".into())))?.to_string();
    let content_type = headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("application/octet-stream").to_string();
    let result = state.service.upload(&principal, &filename, &content_type, &body).await?;
    Ok((StatusCode::CREATED, Json(result)))
}

async fn send_message(State(state): State<Arc<ApiState>>, headers: HeaderMap, Json(request): Json<SendMessageRequest>) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(state.service.send(&principal, request, key).await?))
}

async fn create_draft(State(state): State<Arc<ApiState>>, headers: HeaderMap, Json(request): Json<SendMessageRequest>) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    let key = idempotency_key(&headers)?;
    Ok((StatusCode::CREATED, Json(state.service.draft(&principal, request, key).await?)))
}

async fn reply_message(State(state): State<Arc<ApiState>>, headers: HeaderMap, Path(id): Path<String>, Json(request): Json<ReplyMessageRequest>) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(state.service.reply(&principal, &id, request, key).await?))
}

async fn forward_message(State(state): State<Arc<ApiState>>, headers: HeaderMap, Path(id): Path<String>, Json(request): Json<ForwardMessageRequest>) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(state.service.forward(&principal, &id, request, key).await?))
}

async fn update_message(State(state): State<Arc<ApiState>>, headers: HeaderMap, Path(id): Path<String>, Json(request): Json<MessageUpdateRequest>) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(state.service.update_message(&principal, &id, request).await?))
}

async fn move_message(State(state): State<Arc<ApiState>>, headers: HeaderMap, Path(id): Path<String>, Json(request): Json<MoveMessageRequest>) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(state.service.move_message(&principal, &id, request).await?))
}

async fn archive_message(State(state): State<Arc<ApiState>>, headers: HeaderMap, Path(id): Path<String>) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(state.service.archive_message(&principal, &id).await?))
}

async fn delete_message(State(state): State<Arc<ApiState>>, headers: HeaderMap, Path(id): Path<String>) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(state.service.delete_message(&principal, &id).await?))
}

fn principal(state: &ApiState, headers: &HeaderMap) -> ApiResult<Principal> {
    Ok(state.service.authenticate(bearer(headers)?)?)
}

fn bearer(headers: &HeaderMap) -> ApiResult<&str> {
    let raw = headers.get(AUTHORIZATION).and_then(|v| v.to_str().ok()).ok_or(GatewayError::Authentication)?;
    raw.strip_prefix("Bearer ").filter(|v| !v.is_empty()).ok_or(GatewayError::Authentication).map_err(ApiError)
}

fn idempotency_key(headers: &HeaderMap) -> ApiResult<&str> {
    headers.get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| ApiError(GatewayError::InvalidRequest("Idempotency-Key header is required".into())))
}

fn header_text<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|v| v.to_str().ok()).filter(|v| !v.is_empty())
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
