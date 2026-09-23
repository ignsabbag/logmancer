#![recursion_limit = "256"]

pub mod api;
pub mod app;
pub(crate) mod browser_api_client;
pub mod components;
pub mod file_opening;
#[cfg(feature = "ssr")]
pub mod runtime_parameter;
#[cfg(feature = "ssr")]
pub mod site_root;
mod visual_rules_state;

#[cfg(feature = "ssr")]
#[derive(Debug, PartialEq, Eq)]
pub struct WebServerOptions {
    pub bind_addr: std::net::SocketAddr,
    pub file_root: Option<std::path::PathBuf>,
}

#[cfg(feature = "ssr")]
#[derive(Debug, PartialEq, Eq)]
pub struct ResolvedWebServerOptions {
    pub options: WebServerOptions,
    pub bind_addr_source: ConfigurationSource,
    pub file_root_source: ConfigurationSource,
}

#[cfg(feature = "ssr")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigurationSource {
    Cli,
    Environment,
    Default,
}

#[cfg(feature = "ssr")]
pub fn web_log_target(
    configured_file: Option<std::path::PathBuf>,
    default_directory: std::path::PathBuf,
) -> Result<(std::path::PathBuf, String), String> {
    match configured_file {
        Some(path) => {
            let directory = path
                .parent()
                .filter(|directory| !directory.as_os_str().is_empty())
                .ok_or_else(|| "LOGMANCER_LOG_FILE must include a parent directory".to_string())?;
            let file_name = path
                .file_name()
                .and_then(|file_name| file_name.to_str())
                .filter(|file_name| !file_name.is_empty())
                .ok_or_else(|| "LOGMANCER_LOG_FILE must include a file name".to_string())?;
            Ok((directory.to_path_buf(), file_name.to_owned()))
        }
        None => Ok((default_directory, "logmancer-web.log".to_string())),
    }
}

#[cfg(feature = "ssr")]
pub fn initialize_web_file_logging() -> Result<(), String> {
    use std::io::IsTerminal;

    let default_directory = directories::ProjectDirs::from("dev", "ignsabbag", "Logmancer")
        .map(|directories| directories.data_local_dir().join("logs"))
        .ok_or_else(|| "could not resolve the user data directory".to_string())?;
    let configured_file = std::env::var_os("LOGMANCER_LOG_FILE")
        .filter(|path| !path.to_string_lossy().trim().is_empty())
        .map(std::path::PathBuf::from);

    initialize_web_file_logging_at(
        configured_file,
        default_directory,
        std::io::stderr().is_terminal(),
    )
}

#[cfg(feature = "ssr")]
fn initialize_web_file_logging_at(
    configured_file: Option<std::path::PathBuf>,
    default_directory: std::path::PathBuf,
    stderr_is_terminal: bool,
) -> Result<(), String> {
    let (log_directory, log_file_name) = web_log_target(configured_file, default_directory)?;
    logmancer_core::init_file_logging_with_name(&log_directory, &log_file_name, stderr_is_terminal)
        .map_err(|error| format!("Could not initialize file logging: {error}"))
}

#[cfg(all(test, feature = "ssr"))]
mod logging_tests {
    use super::{initialize_web_file_logging_at, web_log_target};
    use std::path::PathBuf;
    use std::process::Command;

    const CHILD_TEST_ENV: &str = "LOGMANCER_WEB_LOGGING_CHILD_TEST";

    fn run_in_child(test_name: &str) {
        if std::env::var_os(CHILD_TEST_ENV).is_some() {
            return;
        }

        let status = Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg(test_name)
            .arg("--nocapture")
            .env(CHILD_TEST_ENV, "1")
            .status()
            .unwrap();

        assert!(status.success());
    }

    #[test]
    fn configured_log_file_keeps_its_directory_and_name() {
        let configured_file = PathBuf::from("/custom/logs/web.log");

        let target = web_log_target(Some(configured_file), PathBuf::from("/default/logs")).unwrap();

        assert_eq!(
            target,
            (PathBuf::from("/custom/logs"), "web.log".to_string())
        );
    }

    #[test]
    fn default_log_target_uses_the_variant_file_name() {
        let target = web_log_target(None, PathBuf::from("/default/logs")).unwrap();

        assert_eq!(
            target,
            (
                PathBuf::from("/default/logs"),
                "logmancer-web.log".to_string()
            )
        );
    }

    #[test]
    fn configured_log_file_is_written_in_its_parent_directory() {
        if std::env::var_os(CHILD_TEST_ENV).is_none() {
            run_in_child("logging_tests::configured_log_file_is_written_in_its_parent_directory");
            return;
        }

        let directory = tempfile::tempdir().unwrap();
        let configured_file = directory.path().join("custom.log");
        let unrelated_file = directory.path().join("custom.log.unrelated");
        std::fs::write(&unrelated_file, "do not remove").unwrap();

        initialize_web_file_logging_at(
            Some(configured_file),
            directory.path().join("default"),
            false,
        )
        .unwrap();

        assert!(unrelated_file.exists());
        assert!(std::fs::read_dir(directory.path()).unwrap().any(|entry| {
            let path = entry.unwrap().path();
            path != unrelated_file
                && path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("custom.log.")
        }));
        assert!(!directory.path().join("logmancer-logs").exists());
    }

    #[test]
    fn web_logging_reports_invalid_directories() {
        if std::env::var_os(CHILD_TEST_ENV).is_none() {
            run_in_child("logging_tests::web_logging_reports_invalid_directories");
            return;
        }

        let directory = tempfile::tempdir().unwrap();
        let invalid_directory = directory.path().join("not-a-directory");
        std::fs::write(&invalid_directory, "blocked").unwrap();

        let error = initialize_web_file_logging_at(None, invalid_directory, false).unwrap_err();

        assert!(error.starts_with("Could not initialize file logging:"));
    }
}

#[cfg(feature = "ssr")]
impl std::fmt::Display for ConfigurationSource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Cli => "cli",
            Self::Environment => "environment",
            Self::Default => "default",
        })
    }
}

#[cfg(feature = "ssr")]
#[derive(Debug, PartialEq, Eq)]
pub enum WebServerOptionsError {
    Help,
    Usage(String),
}

#[cfg(feature = "ssr")]
impl WebServerOptionsError {
    pub fn is_help(&self) -> bool {
        matches!(self, Self::Help)
    }
}

#[cfg(feature = "ssr")]
impl std::fmt::Display for WebServerOptionsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Help => formatter
                .write_str("Usage: logmancer-web [--bind <socket-address>] [--file-root <path>]"),
            Self::Usage(message) => formatter.write_str(message),
        }
    }
}

#[cfg(feature = "ssr")]
impl std::error::Error for WebServerOptionsError {}

#[cfg(feature = "ssr")]
impl WebServerOptions {
    pub fn from_sources(
        arguments: &[std::ffi::OsString],
        bind_addr_from_env: Option<&str>,
        file_root_from_env: Option<std::ffi::OsString>,
        default_port: u16,
    ) -> Result<Self, WebServerOptionsError> {
        Self::resolve_from_sources(
            arguments,
            bind_addr_from_env,
            file_root_from_env,
            default_port,
        )
        .map(|resolved| resolved.options)
    }

    pub fn resolve_from_sources(
        arguments: &[std::ffi::OsString],
        bind_addr_from_env: Option<&str>,
        file_root_from_env: Option<std::ffi::OsString>,
        default_port: u16,
    ) -> Result<ResolvedWebServerOptions, WebServerOptionsError> {
        let mut bind_addr = None;
        let mut file_root = None;
        let mut arguments = arguments.iter();

        while let Some(argument) = arguments.next() {
            match argument.to_str() {
                Some("--help") | Some("-h") => return Err(WebServerOptionsError::Help),
                Some("--bind") => {
                    let value = arguments.next().ok_or_else(|| {
                        WebServerOptionsError::Usage("--bind requires a socket address".to_string())
                    })?;
                    let value = value.to_str().ok_or_else(|| {
                        WebServerOptionsError::Usage("--bind must be a socket address".to_string())
                    })?;
                    bind_addr = Some(value.parse().map_err(|error| {
                        WebServerOptionsError::Usage(format!(
                            "--bind must be a socket address such as 0.0.0.0:3000: {error}"
                        ))
                    })?);
                }
                Some("--file-root") => {
                    file_root = Some(arguments.next().cloned().ok_or_else(|| {
                        WebServerOptionsError::Usage("--file-root requires a path".to_string())
                    })?);
                }
                Some(option) if option.starts_with('-') => {
                    return Err(WebServerOptionsError::Usage(format!(
                        "Unknown option: {option}"
                    )));
                }
                _ => {
                    return Err(WebServerOptionsError::Usage(
                        "Unexpected argument; use --help for usage.".to_string(),
                    ));
                }
            }
        }

        let (bind_addr, bind_addr_source) = match bind_addr {
            Some(bind_addr) => (bind_addr, ConfigurationSource::Cli),
            None => match bind_addr_from_env
                .map(str::trim)
                .filter(|addr| !addr.is_empty())
            {
                Some(_) => (
                    resolve_bind_addr(bind_addr_from_env, default_port)
                        .map_err(WebServerOptionsError::Usage)?,
                    ConfigurationSource::Environment,
                ),
                None => (
                    resolve_bind_addr(None, default_port).map_err(WebServerOptionsError::Usage)?,
                    ConfigurationSource::Default,
                ),
            },
        };
        let (file_root, file_root_source) = match file_root {
            Some(file_root) => (
                Some(std::path::PathBuf::from(file_root)),
                ConfigurationSource::Cli,
            ),
            None => match file_root_from_env.filter(|path| !path.is_empty()) {
                Some(file_root) => (
                    Some(std::path::PathBuf::from(file_root)),
                    ConfigurationSource::Environment,
                ),
                None => (None, ConfigurationSource::Default),
            },
        };

        Ok(ResolvedWebServerOptions {
            options: Self {
                bind_addr,
                file_root,
            },
            bind_addr_source,
            file_root_source,
        })
    }

    pub fn from_env(
        arguments: &[std::ffi::OsString],
        default_port: u16,
    ) -> Result<Self, WebServerOptionsError> {
        Self::resolve_from_env(arguments, default_port).map(|resolved| resolved.options)
    }

    pub fn resolve_from_env(
        arguments: &[std::ffi::OsString],
        default_port: u16,
    ) -> Result<ResolvedWebServerOptions, WebServerOptionsError> {
        let bind_addr = std::env::var("LOGMANCER_BIND_ADDR").ok();
        Self::resolve_from_sources(
            arguments,
            bind_addr.as_deref(),
            std::env::var_os("LOGMANCER_SERVER_FILE_ROOT"),
            default_port,
        )
    }
}

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
mod web_server_options_contract_tests {
    use super::{ConfigurationSource, WebServerOptions};
    use std::ffi::OsString;
    use std::net::SocketAddr;
    use std::path::PathBuf;

    #[test]
    fn cli_options_override_environment_and_preserve_paths_with_spaces() {
        let options = WebServerOptions::resolve_from_sources(
            &[
                OsString::from("--bind"),
                OsString::from("0.0.0.0:8080"),
                OsString::from("--file-root"),
                OsString::from("/logs with spaces"),
            ],
            Some("127.0.0.1:9000"),
            Some(OsString::from("/environment root")),
            3000,
        )
        .unwrap();

        assert_eq!(
            options.options.bind_addr,
            "0.0.0.0:8080".parse::<SocketAddr>().unwrap()
        );
        assert_eq!(
            options.options.file_root,
            Some(PathBuf::from("/logs with spaces"))
        );
        assert_eq!(options.bind_addr_source, ConfigurationSource::Cli);
        assert_eq!(options.file_root_source, ConfigurationSource::Cli);
    }

    #[test]
    fn environment_options_override_safe_defaults() {
        let options = WebServerOptions::resolve_from_sources(
            &[],
            Some("127.0.0.1:9000"),
            Some(OsString::from("/environment root")),
            3000,
        )
        .unwrap();

        assert_eq!(options.options.bind_addr, "127.0.0.1:9000".parse().unwrap());
        assert_eq!(
            options.options.file_root,
            Some(PathBuf::from("/environment root"))
        );
        assert_eq!(options.bind_addr_source, ConfigurationSource::Environment);
        assert_eq!(options.file_root_source, ConfigurationSource::Environment);

        let defaults =
            WebServerOptions::resolve_from_sources(&[], Some(""), Some(OsString::new()), 3000)
                .unwrap();
        assert_eq!(
            defaults.options.bind_addr,
            "127.0.0.1:3000".parse().unwrap()
        );
        assert_eq!(defaults.options.file_root, None);
        assert_eq!(defaults.bind_addr_source, ConfigurationSource::Default);
        assert_eq!(defaults.file_root_source, ConfigurationSource::Default);
    }

    #[test]
    fn public_options_can_be_constructed_with_bind_address_and_file_root_only() {
        let options = WebServerOptions {
            bind_addr: "127.0.0.1:3000".parse().unwrap(),
            file_root: None,
        };

        assert_eq!(options.bind_addr, "127.0.0.1:3000".parse().unwrap());
        assert_eq!(options.file_root, None);
    }

    #[test]
    fn help_and_invalid_arguments_are_actionable() {
        assert!(
            WebServerOptions::from_sources(&[OsString::from("--help")], None, None, 3000)
                .unwrap_err()
                .is_help()
        );
        for arguments in [
            vec![OsString::from("--bind")],
            vec![OsString::from("--bind"), OsString::from("not-an-address")],
            vec![OsString::from("--file-root")],
            vec![OsString::from("--unknown")],
        ] {
            assert!(WebServerOptions::from_sources(&arguments, None, None, 3000)
                .unwrap_err()
                .to_string()
                .contains("--"));
        }
    }
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
                .with_reader(&file_id, |reader| reader.read_page(0, 1))
                .unwrap()
                .unwrap()
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

        assert!(matches!(
            registry.with_reader(&file_id.unwrap(), |_| ()),
            Ok(Some(()))
        ));
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

    if let Err(error) = initialize_web_file_logging() {
        eprintln!("{error}");
    }

    let file_open_policy = ServerFileRoot::from_env()
        .map(|root| Arc::new(SsrFileOpenPolicy::new(root)) as Arc<dyn FileOpenPolicy>);
    let registry = registry_runtime(config_directory_from_env(), file_open_policy);
    start_leptos_with_registry(addr, registry).await;
}

#[cfg(feature = "ssr")]
pub async fn start_leptos_with_options(options: WebServerOptions) -> Result<(), String> {
    use crate::api::server_browser::{ServerFileRoot, SsrFileOpenPolicy};
    use logmancer_core::FileOpenPolicy;
    use std::sync::Arc;

    initialize_web_file_logging()?;

    let file_root = options
        .file_root
        .as_deref()
        .map(ServerFileRoot::from_path)
        .transpose()
        .map_err(|error| format!("Could not use server file root: {error}"))?;
    let file_open_policy = file_root
        .clone()
        .map(|root| Arc::new(SsrFileOpenPolicy::new(root)) as Arc<dyn FileOpenPolicy>);
    let registry = registry_runtime(config_directory_from_env(), file_open_policy);
    start_leptos_with_registry_inner(options.bind_addr, registry, None, file_root).await;
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn start_leptos_with_registry(
    addr: std::net::SocketAddr,
    registry: std::sync::Arc<logmancer_core::LogRegistry>,
) {
    start_leptos_with_registry_at_site_root(addr, registry, None).await;
}

#[cfg(feature = "ssr")]
pub async fn start_leptos_with_registry_at_site_root(
    addr: std::net::SocketAddr,
    registry: std::sync::Arc<logmancer_core::LogRegistry>,
    desktop_site_root: Option<std::path::PathBuf>,
) {
    start_leptos_with_registry_inner(
        addr,
        registry,
        desktop_site_root,
        crate::api::server_browser::ServerFileRoot::from_env(),
    )
    .await;
}

#[cfg(feature = "ssr")]
async fn start_leptos_with_registry_inner(
    addr: std::net::SocketAddr,
    registry: std::sync::Arc<logmancer_core::LogRegistry>,
    desktop_site_root: Option<std::path::PathBuf>,
    server_file_root: Option<crate::api::server_browser::ServerFileRoot>,
) {
    use crate::api::config::api_routes_with_registry_and_file_root;
    use crate::app::shell;
    use crate::components::App;
    use axum::Router;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use tracing::info;

    let conf = get_configuration(None).unwrap();
    let mut leptos_options = conf.leptos_options;
    let configured_site_root = std::env::var_os("LEPTOS_SITE_ROOT");
    let resolved_site_root = site_root::resolve_site_root_with_parameter_source(
        configured_site_root.clone(),
        runtime_parameter::resolve_runtime_parameter_source(
            configured_site_root.as_deref(),
            std::env::var_os(runtime_parameter::LEPTOS_SITE_ROOT_SOURCE_ENV).as_deref(),
        ),
        desktop_site_root,
        std::env::current_exe().ok().as_deref(),
        std::path::PathBuf::from(leptos_options.site_root.as_ref()),
    );
    leptos_options.site_root = resolved_site_root
        .path
        .to_string_lossy()
        .into_owned()
        .into();
    info!(
        site_root_source = %resolved_site_root.source,
        "Resolved Leptos runtime site root"
    );
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    let app = Router::new()
        .nest(
            "/api",
            api_routes_with_registry_and_file_root(registry.clone(), server_file_root),
        )
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

    if let Err(error) = initialize_web_file_logging() {
        eprintln!("{error}");
    }

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
