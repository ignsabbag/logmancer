#![recursion_limit = "256"]

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use leptos::prelude::*;
    use logmancer_web::init_backend_logging;
    use logmancer_web::{start_leptos, web_bind_addr};
    use tracing::info;

    init_backend_logging();

    let conf = get_configuration(None).unwrap();
    let addr = web_bind_addr(conf.leptos_options.site_addr.port())
        .unwrap_or_else(|error| panic!("Invalid web server configuration: {error}"));
    info!(
        "Launching logmancer-web SSR on {} with LEPTOS_SITE_ROOT={:?} LEPTOS_OUTPUT_NAME={:?}",
        addr,
        std::env::var("LEPTOS_SITE_ROOT").ok(),
        std::env::var("LEPTOS_OUTPUT_NAME").ok()
    );
    start_leptos(addr).await;
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
