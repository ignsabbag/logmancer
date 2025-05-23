use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use leptos::logging::log;
use crate::api::config::AppState;
use crate::api::commons::{ReadPageRequest, TailRequest};

pub async fn read_page(State(app_state): State<AppState>, query: Query<ReadPageRequest>) -> impl IntoResponse {
    log!("payload.path: {:?}", query);

    match app_state.registry.get_reader(&query.file_id) {
        Some(mut reader) => {
            match reader.read_page(query.start_line, query.max_lines) {
                Ok(page_result) => (StatusCode::OK, Json(page_result)).into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Error reading file: {}", e))).into_response()
            }
        }
        None => (StatusCode::NOT_FOUND, Json("File not opened")).into_response()
    }
}

pub async fn tail(State(app_state): State<AppState>, query: Query<TailRequest>) -> impl IntoResponse {
    log!("payload.path: {:?}", query);

    match app_state.registry.get_reader(&query.file_id) {
        Some(mut reader) => {
            match reader.tail(query.max_lines, query.follow) {
                Ok(page_result) => (StatusCode::OK, Json(page_result)).into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Error reading file: {}", e))).into_response()
            }
        }
        None => (StatusCode::NOT_FOUND, Json("File not opened")).into_response()
    }
}
