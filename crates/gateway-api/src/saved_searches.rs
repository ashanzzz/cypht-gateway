use super::{principal, ApiResult, ApiState, DeleteConfirmQuery};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use gateway_core::{SavedSearchCreateRequest, SavedSearchUpdateRequest};
use std::sync::Arc;

pub(super) async fn list(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(state.service.saved_searches(&principal).await?))
}

pub(super) async fn read(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(state.service.saved_search(&principal, &id).await?))
}

pub(super) async fn create(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<SavedSearchCreateRequest>,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok((
        StatusCode::CREATED,
        Json(
            state
                .service
                .create_saved_search(&principal, request)
                .await?,
        ),
    ))
}

pub(super) async fn update(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<SavedSearchUpdateRequest>,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(
        state
            .service
            .update_saved_search(&principal, &id, request)
            .await?,
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
        .delete_saved_search(&principal, &id, query.confirm.unwrap_or(false))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
