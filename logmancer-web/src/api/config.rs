use crate::api::file_info::file_info;
use crate::api::filter::{apply_filter, read_filter_page};
use crate::api::read_page::{read_page, tail};
use crate::api::server_browser::{
    server_browser_list, server_browser_open, server_browser_status, ServerFileRoot,
};
use crate::api::upload_file::upload_file;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::Router;
use logmancer_core::LogRegistry;
use std::sync::Arc;

const LOG_UPLOAD_BODY_LIMIT_BYTES: usize = 512 * 1024 * 1024;

#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<LogRegistry>,
    pub server_file_root: Option<ServerFileRoot>,
}

pub fn api_routes_with_registry<T>(registry: Arc<LogRegistry>) -> Router<T> {
    let server_file_root = ServerFileRoot::from_env();

    Router::new()
        .route("/server-browser/status", get(server_browser_status))
        .route("/server-browser/list", post(server_browser_list))
        .route("/server-browser/open", post(server_browser_open))
        .route("/upload-file", post(upload_file))
        .route("/read-page", get(read_page))
        .route("/file_info", get(file_info))
        .route("/tail", get(tail))
        .route("/apply-filter", post(apply_filter))
        .route("/read-filter-page", get(read_filter_page))
        .layer(DefaultBodyLimit::max(LOG_UPLOAD_BODY_LIMIT_BYTES))
        .with_state(AppState {
            registry,
            server_file_root,
        })
}

pub fn api_routes<T>() -> Router<T> {
    api_routes_with_registry(Arc::new(LogRegistry::new()))
}
