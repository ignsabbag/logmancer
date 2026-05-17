use crate::api::commons::{ApplySearchRequest, SearchNavigateRequest, SearchStatusRequest};
use crate::api::config::AppState;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;

pub async fn apply_search(
    State(app_state): State<AppState>,
    Json(payload): Json<ApplySearchRequest>,
) -> impl IntoResponse {
    match app_state.registry.get_reader(&payload.file_id) {
        Some(mut reader) => match reader.apply_search(payload.query, payload.max_lines) {
            Ok(page_result) => (StatusCode::OK, Json(page_result)).into_response(),
            Err(e) => (
                StatusCode::BAD_REQUEST,
                Json(format!("Error applying search: {e}")),
            )
                .into_response(),
        },
        None => (StatusCode::NOT_FOUND, Json("File not opened")).into_response(),
    }
}

pub async fn clear_search(
    State(app_state): State<AppState>,
    query: Query<SearchStatusRequest>,
) -> impl IntoResponse {
    match app_state.registry.get_reader(&query.file_id) {
        Some(mut reader) => {
            reader.clear_search();
            (StatusCode::OK, Json("Search cleared")).into_response()
        }
        None => (StatusCode::NOT_FOUND, Json("File not opened")).into_response(),
    }
}

pub async fn search_status(
    State(app_state): State<AppState>,
    query: Query<SearchStatusRequest>,
) -> impl IntoResponse {
    match app_state.registry.get_reader(&query.file_id) {
        Some(reader) => (StatusCode::OK, Json(reader.search_status())).into_response(),
        None => (StatusCode::NOT_FOUND, Json("File not opened")).into_response(),
    }
}

pub async fn search_next(
    State(app_state): State<AppState>,
    query: Query<SearchNavigateRequest>,
) -> impl IntoResponse {
    match app_state.registry.get_reader(&query.file_id) {
        Some(mut reader) => match reader.search_next(query.max_lines) {
            Ok(page_result) => (StatusCode::OK, Json(page_result)).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(format!("Error navigating search: {e}")),
            )
                .into_response(),
        },
        None => (StatusCode::NOT_FOUND, Json("File not opened")).into_response(),
    }
}

pub async fn search_previous(
    State(app_state): State<AppState>,
    query: Query<SearchNavigateRequest>,
) -> impl IntoResponse {
    match app_state.registry.get_reader(&query.file_id) {
        Some(mut reader) => match reader.search_previous(query.max_lines) {
            Ok(page_result) => (StatusCode::OK, Json(page_result)).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(format!("Error navigating search: {e}")),
            )
                .into_response(),
        },
        None => (StatusCode::NOT_FOUND, Json("File not opened")).into_response(),
    }
}
