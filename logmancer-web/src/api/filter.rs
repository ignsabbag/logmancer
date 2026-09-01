use crate::api::commons::{ApplyFilterRequest, ReadFilterRequest};
use crate::api::config::{restoration_error_response, AppState};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use tracing::debug;

pub async fn apply_filter(
    State(app_state): State<AppState>,
    Json(payload): Json<ApplyFilterRequest>,
) -> impl IntoResponse {
    debug!(
        "apply_filter: file_id={}, filter={}",
        payload.file_id, payload.filter
    );

    match app_state
        .registry
        .with_reader(&payload.file_id, |reader| reader.filter(payload.filter))
    {
        Ok(Some(())) => (StatusCode::OK, Json("Filter applied")).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json("File not opened")).into_response(),
        Err(error) => restoration_error_response(&error),
    }
}

pub async fn read_filter_page(
    State(app_state): State<AppState>,
    query: Query<ReadFilterRequest>,
) -> impl IntoResponse {
    debug!("read_filter_page: {:?}", query);

    match app_state.registry.with_reader(&query.file_id, |reader| {
        reader.read_filter(query.start_line, query.max_lines)
    }) {
        Ok(Some(result)) => match result {
            Ok(page_result) => (StatusCode::OK, Json(page_result)).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(format!("Error reading filter: {e}")),
            )
                .into_response(),
        },
        Ok(None) => (StatusCode::NOT_FOUND, Json("File not opened")).into_response(),
        Err(error) => restoration_error_response(&error),
    }
}
