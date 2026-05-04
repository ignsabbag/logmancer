use logmancer_web::{init_backend_logging, start_leptos};
use tauri::{Manager, Url};
use tracing::{error, info};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_backend_logging();

    let env = std::env::var("TAURI_ENV_TARGET_TRIPLE").unwrap_or("prd".to_string());
    info!("Starting desktop runtime target={}", env);
    if env == "prd" {
        tauri::Builder::default()
            .setup(|app| {
                let port = std::net::TcpListener::bind("127.0.0.1:0")
                    .expect("Could not open a socket")
                    .local_addr()
                    .expect("The address could not be obtained")
                    .port();
                info!("Spawning embedded SSR server on port={}", port);
                tauri::async_runtime::spawn(async move {
                    info!("Embedded SSR server task started");
                    start_leptos(port).await
                });
                let window = app.get_webview_window("main").unwrap();
                let target = format!("http://127.0.0.1:{}", port);
                info!("Navigating desktop window to {}", target);
                window.navigate(Url::parse(target.as_str()).unwrap())?;
                Ok(())
            })
            .plugin(tauri_plugin_opener::init())
            .run(tauri::generate_context!())
            .expect("Error while running tauri application");
    } else {
        error!("Desktop run skipped because TAURI_ENV_TARGET_TRIPLE is not 'prd'");
    }
}
