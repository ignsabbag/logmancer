use crate::api::commons::{ApplySearchRequest, SearchNavigateRequest, SearchStatusRequest};
use crate::api::config::{restoration_error_response, AppState};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;

pub async fn apply_search(
    State(app_state): State<AppState>,
    Json(payload): Json<ApplySearchRequest>,
) -> impl IntoResponse {
    match app_state.registry.with_reader(&payload.file_id, |reader| {
        reader.apply_search(payload.query, payload.max_lines)
    }) {
        Ok(Some(result)) => match result {
            Ok(page_result) => (StatusCode::OK, Json(page_result)).into_response(),
            Err(e) => (
                StatusCode::BAD_REQUEST,
                Json(format!("Error applying search: {e}")),
            )
                .into_response(),
        },
        Ok(None) => (StatusCode::NOT_FOUND, Json("File not opened")).into_response(),
        Err(error) => restoration_error_response(&error),
    }
}

pub async fn clear_search(
    State(app_state): State<AppState>,
    query: Query<SearchStatusRequest>,
) -> impl IntoResponse {
    match app_state
        .registry
        .with_reader(&query.file_id, |reader| reader.clear_search())
    {
        Ok(Some(())) => (StatusCode::OK, Json("Search cleared")).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json("File not opened")).into_response(),
        Err(error) => restoration_error_response(&error),
    }
}

pub async fn search_status(
    State(app_state): State<AppState>,
    query: Query<SearchStatusRequest>,
) -> impl IntoResponse {
    match app_state
        .registry
        .with_reader(&query.file_id, |reader| reader.search_status())
    {
        Ok(Some(status)) => (StatusCode::OK, Json(status)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json("File not opened")).into_response(),
        Err(error) => restoration_error_response(&error),
    }
}

pub async fn search_next(
    State(app_state): State<AppState>,
    query: Query<SearchNavigateRequest>,
) -> impl IntoResponse {
    match app_state
        .registry
        .with_reader(&query.file_id, |reader| reader.search_next(query.max_lines))
    {
        Ok(Some(result)) => match result {
            Ok(page_result) => (StatusCode::OK, Json(page_result)).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(format!("Error navigating search: {e}")),
            )
                .into_response(),
        },
        Ok(None) => (StatusCode::NOT_FOUND, Json("File not opened")).into_response(),
        Err(error) => restoration_error_response(&error),
    }
}

pub async fn search_previous(
    State(app_state): State<AppState>,
    query: Query<SearchNavigateRequest>,
) -> impl IntoResponse {
    match app_state.registry.with_reader(&query.file_id, |reader| {
        reader.search_previous(query.max_lines)
    }) {
        Ok(Some(result)) => match result {
            Ok(page_result) => (StatusCode::OK, Json(page_result)).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(format!("Error navigating search: {e}")),
            )
                .into_response(),
        },
        Ok(None) => (StatusCode::NOT_FOUND, Json("File not opened")).into_response(),
        Err(error) => restoration_error_response(&error),
    }
}
