use super::{principal, ApiResult, ApiState, DeleteConfirmQuery};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use gateway_core::CalendarEventCreateRequest;
use std::sync::Arc;

#[derive(Debug, serde::Deserialize)]
pub struct CalendarRangeQuery {
    pub start: String,
    pub end: String,
}

pub(super) async fn list(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(vec![state.service.calendar(&principal).await?]))
}

pub(super) async fn events(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(calendar_id): Path<String>,
    Query(query): Query<CalendarRangeQuery>,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(
        state
            .service
            .calendar_events(&principal, &calendar_id, &query.start, &query.end)
            .await?,
    ))
}

pub(super) async fn create(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(calendar_id): Path<String>,
    Json(request): Json<CalendarEventCreateRequest>,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok((
        StatusCode::CREATED,
        Json(
            state
                .service
                .create_calendar_event(&principal, &calendar_id, request)
                .await?,
        ),
    ))
}

pub(super) async fn delete(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((calendar_id, event_id)): Path<(String, String)>,
    Query(query): Query<DeleteConfirmQuery>,
) -> ApiResult<StatusCode> {
    let principal = principal(&state, &headers)?;
    state
        .service
        .delete_calendar_event(
            &principal,
            &calendar_id,
            &event_id,
            query.confirm.unwrap_or(false),
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
