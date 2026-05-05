#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use leptos::prelude::*;
    use logmancer_core::LogRegistry;
    use logmancer_web::init_backend_logging;
    use logmancer_web::start_leptos_with_registry;
    use logmancer_web::try_open_initial_file;
    use std::sync::Arc;
    use tracing::info;

    init_backend_logging();

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let registry = Arc::new(LogRegistry::new());
    let initial_path = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("LOGMANCER_INITIAL_FILE").ok());
    let initial_file_id = try_open_initial_file(&registry, initial_path.as_deref());
    let startup_url = match initial_file_id.as_deref() {
        Some(file_id) => format!("http://{addr}/log/{file_id}"),
        None => format!("http://{addr}"),
    };

    info!(
        "Launching logmancer-web SSR on {} with LEPTOS_SITE_ROOT={:?} LEPTOS_OUTPUT_NAME={:?}",
        addr,
        std::env::var("LEPTOS_SITE_ROOT").ok(),
        std::env::var("LEPTOS_OUTPUT_NAME").ok()
    );
    info!("Initial navigation URL: {}", startup_url);
    start_leptos_with_registry(addr.port(), registry).await;
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
