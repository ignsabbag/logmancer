use logmancer_core::LogRegistry;
use logmancer_web::{
    file_opening::enable_desktop_ssr_runtime, init_backend_logging, start_leptos_with_registry,
    try_open_initial_file,
};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use tauri::{Manager, Url};
use tauri_plugin_dialog::DialogExt;
use tracing::{info, warn};

#[derive(Clone)]
struct DesktopState {
    registry: Arc<LogRegistry>,
}

fn open_selected_log_file(
    registry: &LogRegistry,
    selected_path: Option<PathBuf>,
) -> Result<Option<String>, String> {
    let Some(path) = selected_path else {
        info!("Native file dialog cancelled before selecting a path");
        return Ok(None);
    };

    let path = path.to_string_lossy().to_string();
    info!("Opening native selected file path={}", path);
    match registry.open_file(&path) {
        Ok(file_id) => {
            info!("Native selected file opened file_id={}", file_id);
            Ok(Some(file_id))
        }
        Err(error) => {
            warn!(
                "Could not open native selected file path={} error={}",
                path, error
            );
            Err(format!("Could not open selected file: {error}"))
        }
    }
}

fn wait_for_embedded_server(port: u16) {
    for _ in 0..50 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }

        sleep(Duration::from_millis(50));
    }

    warn!(
        "Embedded SSR server did not accept connections before desktop navigation port={}",
        port
    );
}

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
    open_selected_log_file(state.registry.as_ref(), selected_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn opening_no_selected_path_returns_none_without_requiring_server_root() {
        let registry = LogRegistry::new();

        let result = open_selected_log_file(&registry, None).unwrap();

        assert_eq!(result, None);
    }

    #[test]
    fn opening_selected_path_registers_file_in_shared_registry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.log");
        std::fs::write(&path, "first line\nsecond line").unwrap();
        let registry = LogRegistry::new();

        let file_id = open_selected_log_file(&registry, Some(path))
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
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_backend_logging();
    enable_desktop_ssr_runtime();
    info!("Desktop runtime configured for embedded SSR rendering");
    let initial_path = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("LOGMANCER_INITIAL_FILE").ok());
    info!(initial_path = ?initial_path, "Resolved desktop initial file argument");

    tauri::Builder::default()
        .manage(DesktopState {
            registry: Arc::new(LogRegistry::new()),
        })
        .invoke_handler(tauri::generate_handler![open_native_log_file])
        .setup(move |app| {
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
            let target = match initial_file_id {
                Some(file_id) => format!("http://127.0.0.1:{port}/log/{file_id}"),
                None => format!("http://127.0.0.1:{port}?runtime=desktop"),
            };
            info!("Navigating desktop window to {}", target);
            window.navigate(Url::parse(target.as_str()).unwrap())?;
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("Error while running tauri application");
}
