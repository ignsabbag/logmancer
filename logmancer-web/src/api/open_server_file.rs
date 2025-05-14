use crate::api::config::AppState;
use crate::api::commons::{OpenServerFileRequest, OpenServerFileResponse};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use leptos::logging::log;

pub async fn open_server_file(State(app_state): State<AppState>, Json(payload): Json<OpenServerFileRequest>) -> impl IntoResponse {
    log!("payload.path: {:?}", payload);
    
    match app_state.registry.clone().open_file(&payload.path) {
        Ok(file_id) => {
            log!("Reader created: {}", file_id);
            (StatusCode::CREATED, Json(OpenServerFileResponse {file_id})).into_response()
        }
        Err(e) => {
            log!("Error opening file: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Error opening file: {}", e))).into_response()
        }
    }
}
