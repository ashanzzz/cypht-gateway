use super::{principal, ApiResult, ApiState};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

pub(super) async fn list(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(state.service.feeds_list(&principal).await?))
}

pub(super) async fn read(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(state.service.feed_read(&principal, &id).await?))
}
