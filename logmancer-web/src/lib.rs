#![recursion_limit = "256"]

pub mod api;
pub mod app;
pub(crate) mod browser_api_client;
pub mod components;
pub mod file_opening;
mod visual_rules_state;

#[cfg(feature = "ssr")]
pub fn config_directory_from_env() -> std::path::PathBuf {
    config_directory(
        std::env::var_os("LOGMANCER_CONFIG_DIR").map(std::path::PathBuf::from),
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
    )
}

#[cfg(feature = "ssr")]
pub fn web_bind_addr(default_port: u16) -> Result<std::net::SocketAddr, String> {
    resolve_bind_addr(
        std::env::var("LOGMANCER_BIND_ADDR").ok().as_deref(),
        default_port,
    )
}

#[cfg(feature = "ssr")]
fn resolve_bind_addr(
    configured_addr: Option<&str>,
    default_port: u16,
) -> Result<std::net::SocketAddr, String> {
    match configured_addr
        .map(str::trim)
        .filter(|addr| !addr.is_empty())
    {
        Some(addr) => addr.parse().map_err(|error| {
            format!("LOGMANCER_BIND_ADDR must be a socket address such as 0.0.0.0:3000: {error}")
        }),
        None => Ok(std::net::SocketAddr::from(([127, 0, 0, 1], default_port))),
    }
}

#[cfg(feature = "ssr")]
fn config_directory(
    config_dir: Option<std::path::PathBuf>,
    working_dir: std::path::PathBuf,
) -> std::path::PathBuf {
    config_dir
        .filter(|value| !value.as_os_str().is_empty())
        .unwrap_or_else(|| working_dir.join("config"))
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::{config_directory, registry_runtime, resolve_bind_addr, try_open_initial_file};
    use logmancer_core::{
        LineStyleIntent, ManagedVisualRule, VisualColor, VisualMatcher, VisualRulesEnvelope,
    };
    use std::path::PathBuf;

    #[test]
    fn config_directory_takes_precedence_over_default_location() {
        let path = config_directory(
            Some(PathBuf::from("/temporary/config")),
            PathBuf::from("/working-directory"),
        );

        assert_eq!(path, PathBuf::from("/temporary/config"));
    }

    #[test]
    fn missing_config_directory_uses_the_working_directory_default() {
        let path = config_directory(None, PathBuf::from("/working-directory"));

        assert_eq!(path, PathBuf::from("/working-directory/config"));
    }

    #[test]
    fn missing_bind_address_uses_loopback_with_the_configured_port() {
        let addr = resolve_bind_addr(None, 43123).unwrap();

        assert_eq!(addr, "127.0.0.1:43123".parse().unwrap());
    }

    #[test]
    fn configured_bind_address_overrides_interface_and_port() {
        let addr = resolve_bind_addr(Some("0.0.0.0:8080"), 3000).unwrap();

        assert_eq!(addr, "0.0.0.0:8080".parse().unwrap());
    }

    #[test]
    fn invalid_bind_address_returns_configuration_error() {
        let error = resolve_bind_addr(Some("not-an-address"), 3000).unwrap_err();

        assert!(error.contains("LOGMANCER_BIND_ADDR"));
    }

    #[test]
    fn visual_rules_runtime_creates_parent_and_shares_persisted_rules_with_readers() {
        let directory = tempfile::tempdir().unwrap();
        let config_directory = directory.path().join("config");
        let store_path = config_directory.join("visual-rules.json");
        let log_path = directory.path().join("application.log");
        std::fs::write(&log_path, "ERROR disk\n").unwrap();

        let registry = registry_runtime(config_directory, None);
        let revision = registry.visual_rules_state().revision;
        registry
            .upsert_visual_rules(
                revision,
                VisualRulesEnvelope::new(vec![ManagedVisualRule {
                    name: None,
                    enabled: true,
                    matcher: VisualMatcher::Text("ERROR".to_string()),
                    case_sensitive: true,
                    style: LineStyleIntent {
                        foreground: Some(VisualColor("red".to_string())),
                        background: None,
                    },
                }]),
            )
            .unwrap();
        let file_id = registry.open_file(log_path.to_str().unwrap()).unwrap();

        let mut highlighted = false;
        for _ in 0..100 {
            let page = registry
                .get_reader(&file_id)
                .unwrap()
                .read_page(0, 1)
                .unwrap();
            highlighted = page.lines.first().is_some_and(|line| line.style.is_some());
            if highlighted {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        assert!(store_path.is_file());
        assert!(highlighted);
    }

    #[test]
    fn visual_rules_runtime_survives_load_failure_without_claiming_save_success() {
        let directory = tempfile::tempdir().unwrap();
        let blocked_config_directory = directory.path().join("not-a-directory");
        std::fs::write(&blocked_config_directory, "blocked").unwrap();
        let registry = registry_runtime(blocked_config_directory, None);

        assert!(registry
            .upsert_visual_rules(
                registry.visual_rules_state().revision,
                VisualRulesEnvelope::new(Vec::new())
            )
            .is_err());
    }

    #[test]
    fn shared_runtime_reopens_the_standard_web_initial_file() {
        let directory = tempfile::tempdir().unwrap();
        let log_path = directory.path().join("initial.log");
        std::fs::write(&log_path, "INFO ready\n").unwrap();

        let registry = registry_runtime(directory.path().join("config"), None);
        let file_id = try_open_initial_file(&registry, log_path.to_str());

        assert!(registry.get_reader(&file_id.unwrap()).is_some());
    }
}

#[cfg(feature = "ssr")]
pub fn registry_runtime(
    config_directory: std::path::PathBuf,
    file_open_policy: Option<std::sync::Arc<dyn logmancer_core::FileOpenPolicy>>,
) -> std::sync::Arc<logmancer_core::LogRegistry> {
    use logmancer_core::{ConfigStore, LogRegistry};
    use std::sync::Arc;
    use tracing::warn;

    let config_store = ConfigStore::new(config_directory);
    if let Err(error) = config_store.prepare() {
        warn!(path = %config_store.directory().display(), %error, "Could not prepare configuration directory");
    }
    let mut builder = LogRegistry::builder().config_store(config_store);
    if let Some(file_open_policy) = file_open_policy {
        builder = builder.file_open_policy(file_open_policy);
    }
    let registry = Arc::new(builder.build());
    if let Err(error) = registry.reload_visual_rules() {
        warn!(%error, "Could not load optional visual rules configuration");
    }
    registry
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::components::*;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}

#[cfg(feature = "ssr")]
pub async fn start_leptos(addr: std::net::SocketAddr) {
    use crate::api::server_browser::{ServerFileRoot, SsrFileOpenPolicy};
    use logmancer_core::FileOpenPolicy;
    use std::sync::Arc;

    let file_open_policy = ServerFileRoot::from_env()
        .map(|root| Arc::new(SsrFileOpenPolicy::new(root)) as Arc<dyn FileOpenPolicy>);
    let registry = registry_runtime(config_directory_from_env(), file_open_policy);
    start_leptos_with_registry(addr, registry).await;
}

#[cfg(feature = "ssr")]
pub async fn start_leptos_with_registry(
    addr: std::net::SocketAddr,
    registry: std::sync::Arc<logmancer_core::LogRegistry>,
) {
    start_leptos_with_registry_inner(addr, registry).await;
}

#[cfg(feature = "ssr")]
async fn start_leptos_with_registry_inner(
    addr: std::net::SocketAddr,
    registry: std::sync::Arc<logmancer_core::LogRegistry>,
) {
    use crate::api::config::api_routes_with_registry;
    use crate::app::shell;
    use crate::components::App;
    use axum::Router;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use tracing::info;

    init_backend_logging();

    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    info!(
        "Resolved Leptos runtime config LEPTOS_SITE_ROOT={:?} LEPTOS_OUTPUT_NAME={:?}",
        std::env::var("LEPTOS_SITE_ROOT").ok(),
        std::env::var("LEPTOS_OUTPUT_NAME").ok()
    );
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    let app = Router::new()
        .nest("/api", api_routes_with_registry(registry.clone()))
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
    use tracing::info;

    init_backend_logging();
    let addr = web_bind_addr(port)
        .unwrap_or_else(|error| panic!("Invalid web server configuration: {error}"));

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    info!("Starting API server on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, {
        use crate::api::server_browser::{ServerFileRoot, SsrFileOpenPolicy};
        use logmancer_core::FileOpenPolicy;
        use std::sync::Arc;

        let file_open_policy = ServerFileRoot::from_env()
            .map(|root| Arc::new(SsrFileOpenPolicy::new(root)) as Arc<dyn FileOpenPolicy>);
        let registry = registry_runtime(config_directory_from_env(), file_open_policy);
        api_routes_with_registry(registry).into_make_service()
    })
    .await
    .unwrap();
}

#[cfg(feature = "ssr")]
pub fn try_open_initial_file(
    registry: &std::sync::Arc<logmancer_core::LogRegistry>,
    initial_path: Option<&str>,
) -> Option<String> {
    use tracing::{error, info, warn};

    let path = initial_path
        .map(str::trim)
        .filter(|path| !path.is_empty())?;

    let file_name = std::path::Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<unnamed>");

    info!(file_name = %file_name, "Attempting to open initial file");
    match registry.open_file(path) {
        Ok(file_id) => {
            info!("Initial file opened successfully file_id={}", file_id);
            Some(file_id)
        }
        Err(error) => {
            warn!(file_name = %file_name, %error, "Could not open initial file");
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
                        .with_ansi(false)
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
