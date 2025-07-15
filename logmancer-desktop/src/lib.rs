use logmancer_web::start_leptos;
use tauri::{Manager, Url};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Command::new("printenv").status().ok();
    let env = std::env::var("TAURI_ENV_TARGET_TRIPLE").unwrap_or("prd".to_string());
    println!("Corriendo run {}", env);
    if env == "prd" {
        tauri::Builder::default()
            .setup(|app| {
                let port = std::net::TcpListener::bind("127.0.0.1:0")
                    .expect("Could not open a socket")
                    .local_addr()
                    .expect("The address could not be obtained")
                    .port();
                println!("Corriendo setup");
                tauri::async_runtime::spawn(async move {
                    println!("Corriendo spawn");
                    start_leptos(port).await
                });
                let window = app.get_webview_window("main").unwrap();
                window.navigate(Url::parse(format!("http://127.0.0.1:{}", port).as_str()).unwrap())?;
                Ok(())
            })
            .plugin(tauri_plugin_opener::init())
            .run(tauri::generate_context!())
            .expect("Error while running tauri application");
    }
}
