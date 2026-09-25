use super::{principal, ApiResult, ApiState, DeleteConfirmQuery};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use gateway_core::{TagCreateRequest, TagUpdateRequest};
use std::sync::Arc;

pub(super) async fn list(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(state.service.tags(&principal).await?))
}

pub(super) async fn create(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<TagCreateRequest>,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok((
        StatusCode::CREATED,
        Json(state.service.create_tag(&principal, request).await?),
    ))
}

pub(super) async fn update(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<TagUpdateRequest>,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(
        state.service.update_tag(&principal, &id, request).await?,
    ))
}

pub(super) async fn delete(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<DeleteConfirmQuery>,
) -> ApiResult<StatusCode> {
    let principal = principal(&state, &headers)?;
    state
        .service
        .delete_tag(&principal, &id, query.confirm.unwrap_or(false))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn add_message(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((id, tag_id)): Path<(String, String)>,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(
        state
            .service
            .add_message_tag(&principal, &id, &tag_id)
            .await?,
    ))
}

pub(super) async fn remove_message(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((id, tag_id)): Path<(String, String)>,
    Query(query): Query<DeleteConfirmQuery>,
) -> ApiResult<StatusCode> {
    let principal = principal(&state, &headers)?;
    state
        .service
        .remove_message_tag(&principal, &id, &tag_id, query.confirm.unwrap_or(false))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
