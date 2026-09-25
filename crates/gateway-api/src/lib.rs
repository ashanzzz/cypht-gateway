mod calendar;
mod feeds;
mod saved_searches;
mod sieve;
mod tags;

use axum::{
    body::{Body, Bytes},
    extract::{Path, Query, Request, State},
    http::{
        header::{AUTHORIZATION, CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_TYPE},
        HeaderMap, HeaderValue, StatusCode,
    },
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use gateway_auth::{Principal, DEFAULT_AI_SCOPES};
use gateway_core::{
    ApiErrorBody, ApiErrorDetail, BuildInfo, ContactCreateRequest, ContactUpdateRequest,
    CreateTokenRequest, ForwardMessageRequest, GatewayError, LoginRequest, MeResponse,
    MessageUpdateRequest, MoveMessageRequest, ReplyMessageRequest, SearchRequest,
    SendMessageRequest,
};
use gateway_domain::GatewayService;
use serde::Deserialize;
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tower_http::{
    limit::RequestBodyLimitLayer, set_header::SetResponseHeaderLayer, trace::TraceLayer,
};

tokio::task_local! {
    static REQUEST_ID: String;
}

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

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
        .route("/api/v1/audit", get(audit))
        .route("/api/v1/tokens", get(tokens_list).post(tokens_create))
        .route("/api/v1/tokens/{id}", delete(tokens_revoke))
        .route("/api/v1/accounts", get(accounts))
        .route("/api/v1/contacts", get(contacts).post(contact_create))
        .route(
            "/api/v1/contacts/{id}",
            get(contact_read)
                .patch(contact_update)
                .delete(contact_delete),
        )
        .route(
            "/api/v1/saved-searches",
            get(saved_searches::list).post(saved_searches::create),
        )
        .route(
            "/api/v1/saved-searches/{id}",
            get(saved_searches::read)
                .patch(saved_searches::update)
                .delete(saved_searches::delete),
        )
        .route("/api/v1/calendars", get(calendar::list))
        .route("/api/v1/sieve/status", get(sieve::status))
        .route("/api/v1/feeds", get(feeds::list))
        .route("/api/v1/feeds/{id}", get(feeds::read))
        .route(
            "/api/v1/calendars/{calendar_id}/events",
            get(calendar::events).post(calendar::create),
        )
        .route(
            "/api/v1/calendars/{calendar_id}/events/{event_id}",
            delete(calendar::delete),
        )
        .route("/api/v1/tags", get(tags::list).post(tags::create))
        .route(
            "/api/v1/tags/{id}",
            axum::routing::patch(tags::update).delete(tags::delete),
        )
        .route(
            "/api/v1/messages/{id}/tags/{tag_id}",
            post(tags::add_message).delete(tags::remove_message),
        )
        .route("/api/v1/accounts/{id}/mailboxes", get(mailboxes))
        .route("/api/v1/profiles", get(profiles))
        .route("/api/v1/uploads", post(upload))
        .route("/api/v1/attachments/{id}", get(attachment))
        .route("/api/v1/messages/search", post(search))
        .route("/api/v1/messages/send", post(send_message))
        .route("/api/v1/messages", get(messages))
        .route(
            "/api/v1/messages/{id}",
            get(message).patch(update_message).delete(delete_message),
        )
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
        .layer(middleware::from_fn(request_id_middleware))
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

async fn login(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<LoginRequest>,
) -> ApiResult<impl IntoResponse> {
    Ok((
        StatusCode::OK,
        Json(
            state
                .service
                .login(request.username, request.password)
                .await?,
        ),
    ))
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
        }
        .into(),
        scopes: principal.scopes,
    }))
}

#[derive(Debug, Deserialize)]
struct AuditQuery {
    offset: Option<u64>,
    limit: Option<u32>,
}

async fn audit(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Query(query): Query<AuditQuery>,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(state.service.audit_logs(
        &principal,
        query.offset.unwrap_or(0),
        query.limit.unwrap_or(50),
    )?))
}

async fn tokens_list(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(state.service.list_pats(&principal)?))
}

async fn tokens_create(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Json(mut request): Json<CreateTokenRequest>,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    if request.scopes.is_empty() {
        request.scopes = DEFAULT_AI_SCOPES.iter().map(|s| (*s).to_string()).collect();
    }
    let result = state.service.create_pat(
        &principal,
        &request.name,
        request.scopes,
        request.account_allowlist,
        request.expires_in_days,
    )?;
    Ok((StatusCode::CREATED, Json(result)))
}

async fn tokens_revoke(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let principal = principal(&state, &headers)?;
    state.service.revoke_pat(&principal, &id)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn accounts(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(state.service.accounts(&principal).await?))
}

#[derive(Debug, Deserialize)]
struct ContactsQuery {
    query: Option<String>,
}

async fn contacts(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Query(query): Query<ContactsQuery>,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(
        state
            .service
            .contacts(&principal, query.query.as_deref())
            .await?,
    ))
}

async fn contact_read(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(state.service.contact(&principal, &id).await?))
}

async fn contact_create(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<ContactCreateRequest>,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok((
        StatusCode::CREATED,
        Json(state.service.create_contact(&principal, request).await?),
    ))
}

async fn contact_update(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<ContactUpdateRequest>,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(
        state
            .service
            .update_contact(&principal, &id, request)
            .await?,
    ))
}

#[derive(Debug, Deserialize)]
struct DeleteConfirmQuery {
    confirm: Option<bool>,
}

async fn contact_delete(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<DeleteConfirmQuery>,
) -> ApiResult<StatusCode> {
    let principal = principal(&state, &headers)?;
    state
        .service
        .delete_contact(&principal, &id, query.confirm.unwrap_or(false))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn profiles(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(state.service.profiles(&principal).await?))
}

async fn mailboxes(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
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

async fn messages(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Query(query): Query<MessagesQuery>,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(
        state
            .service
            .messages(
                &principal,
                query.account_id.as_deref(),
                query.mailbox_id.as_deref(),
                query.offset.unwrap_or(0),
                query.limit.unwrap_or(50),
            )
            .await?,
    ))
}

async fn search(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<SearchRequest>,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(state.service.search(&principal, request).await?))
}

async fn message(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(state.service.message(&principal, &id).await?))
}

async fn attachment(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let principal = principal(&state, &headers)?;
    let download = state.service.attachment(&principal, &id).await?;
    let mut builder = Response::builder().status(StatusCode::OK);
    if let Some(content_type) = download.content_type.as_deref() {
        if let Ok(value) = HeaderValue::from_str(content_type) {
            builder = builder.header(CONTENT_TYPE, value);
        }
    }
    if let Some(filename) = download.filename.as_deref() {
        let safe = filename.replace(['\r', '\n', '\"'], "");
        if let Ok(value) = HeaderValue::from_str(&format!("attachment; filename=\"{safe}\"")) {
            builder = builder.header(CONTENT_DISPOSITION, value);
        }
    }
    builder.body(Body::from(download.bytes)).map_err(|e| {
        ApiError(GatewayError::Internal(format!(
            "build attachment response: {e}"
        )))
    })
}

async fn upload(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    let filename = header_text(&headers, "x-filename")
        .ok_or_else(|| {
            ApiError(GatewayError::InvalidRequest(
                "X-Filename header is required".into(),
            ))
        })?
        .to_string();
    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();
    let result = state
        .service
        .upload(&principal, &filename, &content_type, &body)
        .await?;
    Ok((StatusCode::CREATED, Json(result)))
}

async fn send_message(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<SendMessageRequest>,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(state.service.send(&principal, request, key).await?))
}

async fn create_draft(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<SendMessageRequest>,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    let key = idempotency_key(&headers)?;
    Ok((
        StatusCode::CREATED,
        Json(state.service.draft(&principal, request, key).await?),
    ))
}

async fn reply_message(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<ReplyMessageRequest>,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(
        state.service.reply(&principal, &id, request, key).await?,
    ))
}

async fn forward_message(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<ForwardMessageRequest>,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(
        state.service.forward(&principal, &id, request, key).await?,
    ))
}

async fn update_message(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<MessageUpdateRequest>,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(
        state
            .service
            .update_message(&principal, &id, request)
            .await?,
    ))
}

async fn move_message(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<MoveMessageRequest>,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(
        state.service.move_message(&principal, &id, request).await?,
    ))
}

async fn archive_message(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(state.service.archive_message(&principal, &id).await?))
}

async fn delete_message(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(state.service.delete_message(&principal, &id).await?))
}

fn principal(state: &ApiState, headers: &HeaderMap) -> ApiResult<Principal> {
    Ok(state.service.authenticate(bearer(headers)?)?)
}

fn bearer(headers: &HeaderMap) -> ApiResult<&str> {
    let raw = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(GatewayError::Authentication)?;
    raw.strip_prefix("Bearer ")
        .filter(|v| !v.is_empty())
        .ok_or(GatewayError::Authentication)
        .map_err(ApiError)
}

fn idempotency_key(headers: &HeaderMap) -> ApiResult<&str> {
    headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            ApiError(GatewayError::InvalidRequest(
                "Idempotency-Key header is required".into(),
            ))
        })
}

fn header_text<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty())
}

struct ApiError(GatewayError);
type ApiResult<T> = Result<T, ApiError>;

impl From<GatewayError> for ApiError {
    fn from(value: GatewayError) -> Self {
        Self(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self.0 {
            GatewayError::Authentication => StatusCode::UNAUTHORIZED,
            GatewayError::PermissionDenied(_) => StatusCode::FORBIDDEN,
            GatewayError::NotFound(_) => StatusCode::NOT_FOUND,
            GatewayError::Conflict(_) => StatusCode::CONFLICT,
            GatewayError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            GatewayError::PayloadTooLarge(_) => StatusCode::PAYLOAD_TOO_LARGE,
            GatewayError::CapabilityUnavailable(_) => StatusCode::NOT_IMPLEMENTED,
            GatewayError::Upstream(_) => StatusCode::BAD_GATEWAY,
            GatewayError::Configuration(_)
            | GatewayError::Storage(_)
            | GatewayError::Crypto
            | GatewayError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let request_id = REQUEST_ID
            .try_with(Clone::clone)
            .unwrap_or_else(|_| new_request_id());
        let body = ApiErrorBody {
            error: ApiErrorDetail {
                code: self.0.code().into(),
                message: self.0.to_string(),
                request_id: Some(request_id.clone()),
            },
        };
        let mut response = (status, Json(body)).into_response();
        if let Ok(value) = HeaderValue::from_str(&request_id) {
            response.headers_mut().insert("x-request-id", value);
        }
        response
    }
}

async fn request_id_middleware(mut request: Request, next: Next) -> Response {
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .filter(|value| valid_request_id(value))
        .map(str::to_owned)
        .unwrap_or_else(new_request_id);
    if let Ok(value) = HeaderValue::from_str(&request_id) {
        request.headers_mut().insert("x-request-id", value);
    }
    REQUEST_ID
        .scope(request_id.clone(), async move {
            let mut response = next.run(request).await;
            if let Ok(value) = HeaderValue::from_str(&request_id) {
                response.headers_mut().insert("x-request-id", value);
            }
            response
        })
        .await
}

fn valid_request_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn new_request_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let seq = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("req-{millis:x}-{seq:x}")
}

#[cfg(test)]
mod tests;
