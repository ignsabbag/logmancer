use crate::api::file_info::file_info;
use crate::api::filter::{apply_filter, read_filter_page};
use crate::api::open_server_file::open_server_file;
use crate::api::read_page::{read_page, tail};
use axum::routing::{get, post};
use axum::Router;
use logmancer_core::LogRegistry;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<LogRegistry>,
}

pub fn api_routes<T>() -> Router<T> {
    Router::new()
        .route("/open-server-file", post(open_server_file))
        .route("/read-page", get(read_page))
        .route("/file_info", get(file_info))
        .route("/tail", get(tail))
        .route("/apply-filter", post(apply_filter))
        .route("/read-filter-page", get(read_filter_page))
        .with_state(AppState {
            registry: Arc::new(LogRegistry::new()),
        })
}
