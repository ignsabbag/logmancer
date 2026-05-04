use crate::api::commons::{OpenServerFileRequest, OpenServerFileResponse};
use crate::api::config::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use tracing::{error, info, warn};

pub async fn open_server_file(
    State(app_state): State<AppState>,
    Json(payload): Json<OpenServerFileRequest>,
) -> impl IntoResponse {
    let trimmed_path = payload.path.trim();
    if trimmed_path.is_empty() {
        warn!("Rejected open-server-file request with empty path");
        return (
            StatusCode::BAD_REQUEST,
            Json("Path cannot be empty".to_string()),
        )
            .into_response();
    }

    info!("Opening file from API path={}", trimmed_path);

    match app_state.registry.clone().open_file(trimmed_path) {
        Ok(file_id) => {
            info!("Opened file successfully file_id={}", file_id);
            (
                StatusCode::CREATED,
                Json(OpenServerFileResponse { file_id }),
            )
                .into_response()
        }
        Err(e) => {
            error!("Error opening file path={} error={}", trimmed_path, e);
            (
                StatusCode::BAD_REQUEST,
                Json(format!("Could not open file '{}': {}", trimmed_path, e)),
            )
                .into_response()
        }
    }
}
