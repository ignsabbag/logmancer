use logmancer_core::LogRegistry;
use std::net::{TcpStream, ToSocketAddrs};
#[cfg(any(feature = "embedded-server", test))]
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use tauri::{Manager, Url};
use tauri_plugin_dialog::DialogExt;
use tracing::{info, warn};

#[cfg(feature = "embedded-server")]
use {
    logmancer_web::{
        file_opening::enable_desktop_ssr_runtime, start_leptos_with_registry, try_open_initial_file,
    },
    tauri::WindowEvent,
};

#[cfg(not(feature = "embedded-server"))]
const EXTERNAL_DEV_SERVER_URL: &str = "http://localhost:3000?runtime=desktop";

#[cfg(not(feature = "embedded-server"))]
const EXTERNAL_SERVER_FALLBACK_URL: &str = "data:text/html;charset=utf-8,%3C!doctype%20html%3E%3Cmeta%20charset%3Dutf-8%3E%3Ctitle%3ELogmancer%20desktop%3C/title%3E%3Cbody%20style%3D%22font-family%3Asystem-ui%3Bmargin%3A2rem%3Bline-height%3A1.5%22%3E%3Ch1%3ELogmancer%20web%20server%20is%20not%20running%3C/h1%3E%3Cp%3EStart%20it%20with%3A%3C/p%3E%3Cpre%3Ecargo%20leptos%20watch%20--project%20logmancer-web%3C/pre%3E%3Cp%3EThen%20restart%20the%20desktop%20app.%3C/p%3E%3C/body%3E";

#[derive(Clone)]
struct DesktopState {
    #[cfg_attr(not(feature = "embedded-server"), allow(dead_code))]
    registry: Arc<LogRegistry>,
}

fn wait_for_tcp_server<A>(addr: A, attempts: u32, delay: Duration) -> bool
where
    A: ToSocketAddrs + Copy,
{
    for _ in 0..attempts {
        if TcpStream::connect(addr).is_ok() {
            return true;
        }
        sleep(delay);
    }

    false
}

#[cfg(not(feature = "embedded-server"))]
fn wait_for_external_dev_server() -> bool {
    wait_for_tcp_server(("127.0.0.1", 3000), 40, Duration::from_millis(50))
        || wait_for_tcp_server(("localhost", 3000), 10, Duration::from_millis(50))
}

#[cfg(any(feature = "embedded-server", test))]
fn path_basename(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<unnamed>")
        .to_string()
}

#[cfg(any(feature = "embedded-server", test))]
fn open_selected_log_file(
    registry: &LogRegistry,
    selected_path: Option<PathBuf>,
    source: &str,
) -> Result<Option<String>, String> {
    let Some(path) = selected_path else {
        info!(
            source = %source,
            "Native file opening cancelled before selecting a path"
        );
        return Ok(None);
    };

    let file_name = path_basename(&path);
    let path = path.to_string_lossy().to_string();
    info!(source = %source, file_name = %file_name, "Opening native selected file");
    match registry.open_file(&path) {
        Ok(file_id) => {
            info!(source = %source, file_name = %file_name, file_id, "Native selected file opened");
            Ok(Some(file_id))
        }
        Err(error) => {
            warn!(
                source = %source,
                file_name = %file_name, %error,
                "Could not open native selected file"
            );
            Err(format!("Could not open selected file: {error}"))
        }
    }
}

#[cfg(any(feature = "embedded-server", test))]
fn open_dropped_log_path(registry: &LogRegistry, selected_path: PathBuf) -> Result<String, String> {
    let file_name = path_basename(&selected_path);
    info!(file_name = %file_name, "Native dropped path received");

    open_selected_log_file(registry, Some(selected_path), "native_drop")?
        .ok_or_else(|| "Dropped file path was not provided.".to_string())
}

#[cfg(any(feature = "embedded-server", test))]
fn open_first_dropped_log_path(
    registry: &LogRegistry,
    paths: Vec<PathBuf>,
) -> Result<Option<String>, String> {
    let Some(path) = paths.into_iter().next() else {
        warn!("Native drop event did not include file paths");
        return Ok(None);
    };

    open_dropped_log_path(registry, path).map(Some)
}

#[cfg(feature = "embedded-server")]
#[tauri::command]
async fn open_native_log_file(
    app: tauri::AppHandle,
    state: tauri::State<'_, DesktopState>,
) -> Result<Option<String>, String> {
    info!("Native open command invoked from webview");
    let selected_path = app
        .dialog()
        .file()
        .blocking_pick_file()
        .map(|path| {
            path.into_path()
                .map_err(|error| format!("Could not resolve selected file path: {error}"))
        })
        .transpose()?;
    open_selected_log_file(state.registry.as_ref(), selected_path, "native_picker")
}

#[cfg(not(feature = "embedded-server"))]
#[tauri::command]
async fn open_native_log_file(
    app: tauri::AppHandle,
    _state: tauri::State<'_, DesktopState>,
) -> Result<Option<String>, String> {
    info!("Native open command invoked from webview (external server mode)");
    let selected_path = app
        .dialog()
        .file()
        .blocking_pick_file()
        .map(|path| {
            path.into_path()
                .map_err(|error| format!("Could not resolve selected file path: {error}"))
        })
        .transpose()?;
    match selected_path {
        Some(path) => {
            let path_str = path.to_string_lossy().to_string();
            info!(
                "Returning file path for external server (filename={})",
                path.file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or_default()
            );
            // Prefix with "path:" so the WASM code can distinguish from a UUID file_id
            Ok(Some(format!("path:{}", path_str)))
        }
        None => {
            info!("Native file open cancelled");
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn opening_no_selected_path_returns_none_without_requiring_server_root() {
        let registry = LogRegistry::new();

        let result = open_selected_log_file(&registry, None, "test").unwrap();

        assert_eq!(result, None);
    }

    #[test]
    fn opening_selected_path_registers_file_in_shared_registry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.log");
        std::fs::write(&path, "first line\nsecond line").unwrap();
        let registry = LogRegistry::new();

        let file_id = open_selected_log_file(&registry, Some(path), "test")
            .unwrap()
            .unwrap();
        let reader = registry.get_reader(&file_id).unwrap();

        for _ in 0..10 {
            if reader.file_info().unwrap().total_lines >= 2 {
                break;
            }
            sleep(Duration::from_millis(50));
        }

        let info = reader.file_info().unwrap();

        assert_eq!(info.total_lines, 2);
    }

    #[test]
    fn opening_selected_path_returns_error_for_missing_file() {
        let registry = LogRegistry::new();

        let error = open_selected_log_file(
            &registry,
            Some(PathBuf::from("/tmp/logmancer-missing-review-test.log")),
            "test",
        )
        .unwrap_err();

        assert!(error.contains("Could not open selected file"));
    }

    #[test]
    fn opening_dropped_path_registers_file_in_shared_registry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dropped.log");
        std::fs::write(&path, "dropped line").unwrap();
        let registry = LogRegistry::new();

        let file_id = open_dropped_log_path(&registry, path).unwrap();

        assert!(registry.get_reader(&file_id).is_some());
    }

    #[test]
    fn opening_first_dropped_path_uses_first_path() {
        let dir = tempfile::tempdir().unwrap();
        let first_path = dir.path().join("first.log");
        let second_path = dir.path().join("second.log");
        std::fs::write(&first_path, "first line").unwrap();
        std::fs::write(&second_path, "second line").unwrap();
        let registry = LogRegistry::new();

        let file_id = open_first_dropped_log_path(&registry, vec![first_path, second_path])
            .unwrap()
            .unwrap();
        let info = registry.get_reader(&file_id).unwrap().file_info().unwrap();

        assert!(info.path.ends_with("first.log"));
    }

    #[test]
    fn opening_first_dropped_path_returns_none_for_empty_drop() {
        let registry = LogRegistry::new();

        let file_id = open_first_dropped_log_path(&registry, Vec::new()).unwrap();

        assert_eq!(file_id, None);
    }

    #[test]
    fn default_capability_keeps_native_open_inside_tauri_boundary() {
        let capability = include_str!("../capabilities/default.json");

        assert!(capability.contains("http://127.0.0.1:*"));
        assert!(capability.contains("http://localhost:*"));
        assert!(capability.contains("dialog:allow-open"));
        assert!(capability.contains("opener:default"));
        assert!(!capability.contains("fs:"));
        assert!(!capability.contains("shell:"));
    }

    #[test]
    fn wait_for_tcp_server_detects_available_listener() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        assert!(wait_for_tcp_server(addr, 1, Duration::ZERO));
    }

    #[test]
    fn wait_for_tcp_server_returns_false_when_unavailable() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        assert!(!wait_for_tcp_server(addr, 1, Duration::ZERO));
    }
}

fn init_desktop_logging() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,logmancer_desktop=debug"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .try_init();
}

#[cfg(feature = "embedded-server")]
fn wait_for_embedded_server(port: u16) {
    if wait_for_tcp_server(("127.0.0.1", port), 50, Duration::from_millis(50)) {
        return;
    }

    warn!(
        "Embedded SSR server did not accept connections before desktop navigation port={}",
        port
    );
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_desktop_logging();
    let initial_path = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("LOGMANCER_INITIAL_FILE").ok());
    info!(
        initial_file_provided = initial_path.is_some(),
        "Resolved desktop initial file argument"
    );

    let builder = tauri::Builder::default()
        .manage(DesktopState {
            registry: Arc::new(LogRegistry::new()),
        })
        .invoke_handler(tauri::generate_handler![open_native_log_file])
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init());

    #[cfg(feature = "embedded-server")]
    let builder = builder.setup(move |app| {
        enable_desktop_ssr_runtime();
        info!("Desktop runtime configured for embedded SSR rendering");

        let registry = app.state::<DesktopState>().registry.clone();
        let initial_file_id = try_open_initial_file(&registry, initial_path.as_deref());
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .expect("Could not open a socket")
            .local_addr()
            .expect("The address could not be obtained")
            .port();
        info!("Spawning embedded SSR server on port={}", port);
        tauri::async_runtime::spawn(async move {
            info!("Embedded SSR server task started");
            start_leptos_with_registry(port, registry).await
        });
        wait_for_embedded_server(port);
        let window = app.get_webview_window("main").unwrap();
        let registry_for_drop = app.state::<DesktopState>().registry.clone();
        let window_for_drop = window.clone();
        window.on_window_event(move |event| match event {
            WindowEvent::DragDrop(tauri::DragDropEvent::Enter { paths, .. }) => {
                info!(
                    path_count = paths.len(),
                    "Native file drop entered desktop window"
                );
            }
            WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) => {
                info!(
                    path_count = paths.len(),
                    "Native file drop completed on desktop window"
                );
                match open_first_dropped_log_path(registry_for_drop.as_ref(), paths.clone()) {
                    Ok(Some(file_id)) => {
                        let target = format!(
                            "http://127.0.0.1:{port}/log/{file_id}?runtime=desktop-embedded"
                        );
                        info!(file_id, "Navigating after native file drop");
                        if let Err(error) = window_for_drop.navigate(Url::parse(&target).unwrap()) {
                            warn!(%error, "Could not navigate after native file drop");
                        }
                    }
                    Ok(None) => warn!("Native file drop completed without file paths"),
                    Err(error) => warn!(%error, "Could not open native dropped file"),
                }
            }
            WindowEvent::DragDrop(tauri::DragDropEvent::Over { .. }) => {}
            WindowEvent::DragDrop(tauri::DragDropEvent::Leave) => {
                info!("Native file drop left desktop window");
            }
            _ => {}
        });
        let target = match initial_file_id {
            Some(file_id) => {
                format!("http://127.0.0.1:{port}/log/{file_id}?runtime=desktop-embedded")
            }
            None => format!("http://127.0.0.1:{port}?runtime=desktop-embedded"),
        };
        info!("Navigating desktop window to {}", target);
        window.navigate(Url::parse(target.as_str()).unwrap())?;
        Ok(())
    });

    #[cfg(not(feature = "embedded-server"))]
    let builder = builder.setup(move |app| {
        let window = app.get_webview_window("main").unwrap();
        if wait_for_external_dev_server() {
            info!(
                "External server mode: navigating desktop window to {}",
                EXTERNAL_DEV_SERVER_URL
            );
            window.navigate(Url::parse(EXTERNAL_DEV_SERVER_URL)?)?;
        } else {
            warn!(
                "External server mode could not reach localhost:3000. Start it with `cargo leptos watch --project logmancer-web`, then restart the desktop app."
            );
            if let Err(error) = window.navigate(Url::parse(EXTERNAL_SERVER_FALLBACK_URL)?) {
                warn!(%error, "Could not navigate to external server fallback page");
            }
        }
        Ok(())
    });

    builder
        .run(tauri::generate_context!())
        .expect("Error while running tauri application");
}
