use super::{principal, ApiResult, ApiState};
use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};
use std::sync::Arc;

pub(super) async fn status(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> ApiResult<impl IntoResponse> {
    let principal = principal(&state, &headers)?;
    Ok(Json(state.service.sieve_status(&principal).await?))
}
