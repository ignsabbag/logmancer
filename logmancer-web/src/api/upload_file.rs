use crate::api::commons::OpenServerFileResponse;
use crate::api::config::AppState;
use axum::extract::{Multipart, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, info, warn};

pub async fn upload_file(
    State(app_state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut temp_path: Option<PathBuf> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() != Some("file") {
            continue;
        }

        let file_name = field
            .file_name()
            .map(|name| name.replace(['/', '\\'], "_"))
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "uploaded.log".to_string());

        let bytes = match field.bytes().await {
            Ok(bytes) => bytes,
            Err(err) => {
                error!("Error reading multipart bytes: {}", err);
                return (
                    StatusCode::BAD_REQUEST,
                    Json("Could not read uploaded file.".to_string()),
                )
                    .into_response();
            }
        };

        if bytes.is_empty() {
            warn!("Rejected upload-file request with empty payload");
            return (
                StatusCode::BAD_REQUEST,
                Json("Uploaded file cannot be empty.".to_string()),
            )
                .into_response();
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let mut path = std::env::temp_dir();
        path.push(format!("logmancer-upload-{}-{}", timestamp, file_name));

        if let Err(err) = std::fs::write(&path, &bytes) {
            error!("Error writing temp uploaded file path={:?} error={}", path, err);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json("Could not store temporary uploaded file.".to_string()),
            )
                .into_response();
        }

        temp_path = Some(path);
        break;
    }

    let Some(path) = temp_path else {
        warn!("Rejected upload-file request without file field");
        return (
            StatusCode::BAD_REQUEST,
            Json("Upload request is missing the file field.".to_string()),
        )
            .into_response();
    };

    let path_string = path.to_string_lossy().to_string();
    info!("Opening uploaded temp file path={}", path_string);

    match app_state.registry.clone().open_file(&path_string) {
        Ok(file_id) => (
            StatusCode::CREATED,
            Json(OpenServerFileResponse { file_id }),
        )
            .into_response(),
        Err(err) => {
            error!("Error opening uploaded file path={} error={}", path_string, err);
            (
                StatusCode::BAD_REQUEST,
                Json(format!("Could not open uploaded file: {}", err)),
            )
                .into_response()
        }
    }
}
