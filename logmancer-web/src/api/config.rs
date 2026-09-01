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

pub(crate) fn restoration_error_response(error: &std::io::Error) -> axum::response::Response {
    let (status, message) = match error.kind() {
        std::io::ErrorKind::NotFound => (
            axum::http::StatusCode::NOT_FOUND,
            "The persisted file is no longer available",
        ),
        std::io::ErrorKind::PermissionDenied => (
            axum::http::StatusCode::FORBIDDEN,
            "Access to the persisted file is not permitted",
        ),
        _ => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Could not restore the persisted file",
        ),
    };
    axum::response::IntoResponse::into_response((status, axum::Json(message)))
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
    use axum::body::{to_bytes, Body};
    use axum::http::{Method, Request, StatusCode};
    use logmancer_core::{ConfigStore, FileOpenPolicy, VisualRulesEnvelope};
    use std::io;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use tower::ServiceExt;

    struct RejectPolicy;

    impl FileOpenPolicy for RejectPolicy {
        fn validate(&self, _path: &Path) -> io::Result<PathBuf> {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "persisted path is not authorized",
            ))
        }
    }

    struct FailingPolicy;

    impl FileOpenPolicy for FailingPolicy {
        fn validate(&self, _path: &Path) -> io::Result<PathBuf> {
            Err(io::Error::other("internal policy failure"))
        }
    }

    fn visual_rules_router() -> Router {
        let directory = tempfile::tempdir().unwrap().keep();
        let config_store = ConfigStore::new(directory);
        config_store.prepare().unwrap();
        let registry = Arc::new(LogRegistry::builder().config_store(config_store).build());
        registry.reload_visual_rules().unwrap();
        api_routes_with_registry(registry)
    }

    fn persisted_file_id(config: &Path, path: &Path) -> String {
        let store = ConfigStore::new(config.to_path_buf());
        store.prepare().unwrap();
        let registry = LogRegistry::builder().config_store(store).build();
        registry.open_file(path.to_str().unwrap()).unwrap()
    }

    fn restored_registry(
        config: &Path,
        file_open_policy: Option<Arc<dyn FileOpenPolicy>>,
    ) -> Arc<LogRegistry> {
        let store = ConfigStore::new(config.to_path_buf());
        store.prepare().unwrap();
        let mut builder = LogRegistry::builder().config_store(store);
        if let Some(file_open_policy) = file_open_policy {
            builder = builder.file_open_policy(file_open_policy);
        }
        Arc::new(builder.build())
    }

    async fn response_body(response: axum::response::Response) -> String {
        String::from_utf8(
            to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap()
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
    async fn persisted_restoration_errors_have_safe_actionable_http_responses() {
        let directory = tempfile::tempdir().unwrap();
        let config = directory.path().join("config");
        let missing_path = directory.path().join("sensitive-missing.log");
        std::fs::write(&missing_path, "INFO ready\n").unwrap();
        let missing_id = persisted_file_id(&config, &missing_path);
        std::fs::remove_file(&missing_path).unwrap();
        let missing_response = api_routes_with_registry::<()>(restored_registry(&config, None))
            .oneshot(
                Request::builder()
                    .uri(format!("/file_info?file_id={missing_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let missing_status = missing_response.status();
        let missing_body = response_body(missing_response).await;
        assert_eq!(missing_status, StatusCode::NOT_FOUND);
        assert_eq!(
            missing_body,
            "\"The persisted file is no longer available\""
        );

        let restricted_path = directory.path().join("sensitive-restricted.log");
        std::fs::write(&restricted_path, "INFO ready\n").unwrap();
        let restricted_id = persisted_file_id(&config, &restricted_path);
        let restricted_response = api_routes_with_registry::<()>(restored_registry(
            &config,
            Some(Arc::new(RejectPolicy)),
        ))
        .oneshot(
            Request::builder()
                .uri(format!("/file_info?file_id={restricted_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
        let restricted_status = restricted_response.status();
        let restricted_body = response_body(restricted_response).await;
        assert_eq!(restricted_status, StatusCode::FORBIDDEN);
        assert_eq!(
            restricted_body,
            "\"Access to the persisted file is not permitted\""
        );

        let failed_path = directory.path().join("sensitive-failed.log");
        std::fs::write(&failed_path, "INFO ready\n").unwrap();
        let failed_id = persisted_file_id(&config, &failed_path);
        let failed_response = api_routes_with_registry::<()>(restored_registry(
            &config,
            Some(Arc::new(FailingPolicy)),
        ))
        .oneshot(
            Request::builder()
                .uri(format!("/file_info?file_id={failed_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
        let failed_status = failed_response.status();
        let failed_body = response_body(failed_response).await;
        assert_eq!(failed_status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(failed_body, "\"Could not restore the persisted file\"");

        for path in [&missing_path, &restricted_path, &failed_path] {
            assert!(!missing_body.contains(&path.display().to_string()));
            assert!(!restricted_body.contains(&path.display().to_string()));
            assert!(!failed_body.contains(&path.display().to_string()));
        }
    }

    #[tokio::test]
    async fn absent_file_id_remains_a_generic_not_found() {
        let response = api_routes_with_registry::<()>(Arc::new(LogRegistry::new()))
            .oneshot(
                Request::builder()
                    .uri("/file_info?file_id=00000000-0000-0000-0000-000000000000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(response_body(response).await, "\"File not opened\"");
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
