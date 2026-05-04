#[cfg(feature = "ssr")]
mod api;

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use leptos::prelude::*;
    use logmancer_web::init_backend_logging;
    use logmancer_web::start_leptos;
    use tracing::info;

    init_backend_logging();

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    info!(
        "Launching logmancer-web SSR on {} with LEPTOS_SITE_ROOT={:?} LEPTOS_OUTPUT_NAME={:?}",
        addr,
        std::env::var("LEPTOS_SITE_ROOT").ok(),
        std::env::var("LEPTOS_OUTPUT_NAME").ok()
    );
    start_leptos(addr.port()).await;
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
