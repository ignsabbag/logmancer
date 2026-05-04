pub mod api;
pub mod app;
pub mod components;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::components::*;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}

#[cfg(feature = "ssr")]
pub async fn start_leptos(port: u16) {
    use logmancer_core::LogRegistry;
    use std::sync::Arc;

    start_leptos_with_registry(port, Arc::new(LogRegistry::new())).await;
}

#[cfg(feature = "ssr")]
pub async fn start_leptos_with_registry(
    port: u16,
    registry: std::sync::Arc<logmancer_core::LogRegistry>,
) {
    use crate::api::config::api_routes_with_registry;
    use crate::app::shell;
    use crate::components::App;
    use axum::Router;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use std::net::SocketAddr;
    use tracing::info;

    init_backend_logging();

    let conf = get_configuration(None).unwrap();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let leptos_options = conf.leptos_options;
    info!(
        "Resolved Leptos runtime config LEPTOS_SITE_ROOT={:?} LEPTOS_OUTPUT_NAME={:?}",
        std::env::var("LEPTOS_SITE_ROOT").ok(),
        std::env::var("LEPTOS_OUTPUT_NAME").ok()
    );
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    let app = Router::new()
        .nest("/api", api_routes_with_registry(registry))
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    info!("Starting Leptos SSR server on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(feature = "ssr")]
pub async fn start_axum(port: u16) {
    use crate::api::config::api_routes_with_registry;
    use logmancer_core::LogRegistry;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use tracing::info;

    init_backend_logging();

    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    info!("Starting API server on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(
        listener,
        api_routes_with_registry(Arc::new(LogRegistry::new())).into_make_service(),
    )
    .await
    .unwrap();
}

#[cfg(feature = "ssr")]
pub fn try_open_initial_file(
    registry: &std::sync::Arc<logmancer_core::LogRegistry>,
    initial_path: Option<&str>,
) -> Option<String> {
    use tracing::{error, info, warn};

    let Some(path) = initial_path.map(str::trim).filter(|path| !path.is_empty()) else {
        return None;
    };

    info!("Attempting to open initial file path={}", path);
    match registry.open_file(path) {
        Ok(file_id) => {
            info!("Initial file opened successfully file_id={}", file_id);
            Some(file_id)
        }
        Err(error) => {
            warn!("Could not open initial file path={} error={}", path, error);
            error!("Continuing startup without initial file");
            None
        }
    }
}

#[cfg(feature = "ssr")]
pub fn init_backend_logging() {
    use std::path::PathBuf;
    use std::sync::{Once, OnceLock};
    use tracing_appender::non_blocking::WorkerGuard;
    use tracing_subscriber::{fmt, EnvFilter};

    static INIT: Once = Once::new();
    static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

    INIT.call_once(|| {
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info,logmancer_web=debug,logmancer_desktop=debug"));

        if let Ok(log_file) = std::env::var("LOGMANCER_LOG_FILE") {
            if !log_file.trim().is_empty() {
                let log_path = PathBuf::from(log_file);
                if let Some(parent) = log_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }

                if let (Some(parent), Some(file_name)) = (log_path.parent(), log_path.file_name()) {
                    let file_appender =
                        tracing_appender::rolling::never(parent, PathBuf::from(file_name));
                    let (writer, guard) = tracing_appender::non_blocking(file_appender);
                    let _ = LOG_GUARD.set(guard);
                    let _ = fmt()
                        .with_env_filter(env_filter)
                        .with_writer(writer)
                        .with_target(false)
                        .try_init();
                    return;
                }
            }
        }

        let _ = fmt()
            .with_env_filter(env_filter)
            .with_target(false)
            .try_init();
    });
}
