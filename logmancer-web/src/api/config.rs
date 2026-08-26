use crate::api::file_info::file_info;
use crate::api::filter::{apply_filter, read_filter_page};
use crate::api::read_page::{read_page, tail};
use crate::api::search::{apply_search, clear_search, search_next, search_previous, search_status};
use crate::api::server_browser::{
    server_browser_list, server_browser_open, server_browser_status, ServerFileRoot,
};
use crate::api::upload_file::upload_file;
use crate::api::visual_rules::{get_visual_rules, retry_visual_rules, save_visual_rules};
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
        .route("/apply-search", post(apply_search))
        .route("/clear-search", get(clear_search))
        .route("/search-status", get(search_status))
        .route("/search-next", get(search_next))
        .route("/search-previous", get(search_previous))
        .route("/visual-rules", get(get_visual_rules))
        .route("/visual-rules/save", post(save_visual_rules))
        .route("/visual-rules/retry", post(retry_visual_rules))
        .layer(DefaultBodyLimit::max(LOG_UPLOAD_BODY_LIMIT_BYTES))
        .with_state(AppState {
            registry,
            server_file_root,
        })
}

pub fn api_routes<T>() -> Router<T> {
    api_routes_with_registry(Arc::new(LogRegistry::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use logmancer_core::{ConfigStore, VisualRulesEnvelope};
    use std::sync::Arc;
    use tower::ServiceExt;

    fn visual_rules_router() -> Router {
        let directory = tempfile::tempdir().unwrap().keep();
        let config_store = ConfigStore::new(directory);
        config_store.prepare().unwrap();
        let registry = Arc::new(LogRegistry::builder().config_store(config_store).build());
        registry.reload_visual_rules().unwrap();
        api_routes_with_registry(registry)
    }

    #[tokio::test]
    async fn normal_web_router_does_not_register_open_server_file() {
        let response = api_routes_with_registry::<()>(Arc::new(LogRegistry::new()))
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/open-server-file")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn visual_rules_rejects_wrong_method_unknown_route_and_malformed_body_without_mutation() {
        let router = visual_rules_router();
        let wrong_method = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/visual-rules/save")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(wrong_method.status(), StatusCode::METHOD_NOT_ALLOWED);

        let unknown = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/visual-rules/unknown")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(unknown.status(), StatusCode::NOT_FOUND);

        let malformed = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/visual-rules/save")
                    .header("content-type", "application/json")
                    .body(Body::from("not json"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/visual-rules")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn visual_rules_rejects_stale_revision_without_mutating_saved_envelope() {
        let router = visual_rules_router();
        let body = serde_json::json!({
            "baseRevision": 99,
            "envelope": VisualRulesEnvelope::new(Vec::new()),
        })
        .to_string();

        let response = router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/visual-rules/save")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn development_visual_rules_example_uses_the_current_envelope_schema() {
        let envelope: VisualRulesEnvelope =
            serde_json::from_str(include_str!("../../../examples/visual-rules.dev.json")).unwrap();

        let report = envelope.validate_for_save().unwrap();
        assert_eq!(envelope.schema_version, 1);
        assert_eq!(report.evaluator_rules.len(), 2);
    }
}
