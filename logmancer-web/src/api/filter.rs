use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use crate::api::config::AppState;
use crate::api::commons::{ApplyFilterRequest, ReadFilterRequest};
use tracing::debug;

pub async fn apply_filter(State(app_state): State<AppState>, Json(payload): Json<ApplyFilterRequest>) -> impl IntoResponse {
    debug!("apply_filter: file_id={}, filter={}", payload.file_id, payload.filter);

    match app_state.registry.get_reader(&payload.file_id) {
        Some(mut reader) => {
            reader.filter(payload.filter);
            (StatusCode::OK, Json("Filter applied")).into_response()
        }
        None => (StatusCode::NOT_FOUND, Json("File not opened")).into_response()
    }
}

pub async fn read_filter_page(State(app_state): State<AppState>, query: Query<ReadFilterRequest>) -> impl IntoResponse {
    debug!("read_filter_page: {:?}", query);

    match app_state.registry.get_reader(&query.file_id) {
        Some(mut reader) => {
            match reader.read_filter(query.start_line, query.max_lines) {
                Ok(page_result) => (StatusCode::OK, Json(page_result)).into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Error reading filter: {}", e))).into_response()
            }
        }
        None => (StatusCode::NOT_FOUND, Json("File not opened")).into_response()
    }
}
