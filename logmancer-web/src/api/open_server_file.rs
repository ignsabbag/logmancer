use crate::api::app_state::AppState;
use crate::api::commons::{OpenServerFileRequest, OpenServerFileResponse};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use leptos::logging::log;
use logmancer_core::LogReader;

pub async fn open_server_file(State(app_state): State<AppState>, Json(payload): Json<OpenServerFileRequest>) -> impl IntoResponse {
    log!("payload.path: {:?}", payload);

    match LogReader::new(payload.path.clone()) {
        Ok(reader) => {
            log!("Reader created");
            let id = "1";
            let prev = app_state.open_files.readers.insert(id.to_string(), reader);

            if prev.is_none() {
                log!("Reader inserted");
                (StatusCode::CREATED, Json(OpenServerFileResponse {file_id: id.to_string()})).into_response()
            } else {
                log!("Reader NOT inserted");
                (StatusCode::CONFLICT, Json("UUID already exists".to_string())).into_response()
            }
        }
        Err(e) => {
            log!("Error opening file: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Error opening file: {}", e))).into_response()
        }
    }
}
