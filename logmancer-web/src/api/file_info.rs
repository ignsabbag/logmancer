use crate::api::commons::FileInfoRequest;
use crate::api::config::{restoration_error_response, AppState};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use tracing::debug;

pub async fn file_info(
    State(app_state): State<AppState>,
    query: Query<FileInfoRequest>,
) -> impl IntoResponse {
    debug!("Getting info about: {:?}", query);
    match app_state
        .registry
        .with_reader(&query.file_id, |reader| reader.file_info())
    {
        Ok(Some(result)) => match result {
            Ok(file_info) => (StatusCode::OK, Json(file_info)).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(format!("Error reading file: {e}")),
            )
                .into_response(),
        },
        Ok(None) => (StatusCode::NOT_FOUND, Json("File not opened")).into_response(),
        Err(error) => restoration_error_response(&error),
    }
}
