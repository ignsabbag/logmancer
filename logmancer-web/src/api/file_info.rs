use crate::api::commons::FileInfoRequest;
use crate::api::config::AppState;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use leptos::logging::log;

pub async fn file_info(State(app_state): State<AppState>, query: Query<FileInfoRequest>) -> impl IntoResponse {
    log!("Getting info about: {:?}", query);
    match app_state.registry.get_reader(&query.file_id) {
        Some(reader) => {
            match reader.file_info() {
                Ok(file_info) => (StatusCode::OK, Json(file_info)).into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Error reading file: {}", e))).into_response()
            }
        }
        None => (StatusCode::NOT_FOUND, Json("File not opened")).into_response()
    }
}
