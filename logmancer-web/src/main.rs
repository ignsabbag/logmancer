#[cfg(feature = "ssr")]
mod api;

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use leptos::prelude::*;
    use logmancer_web::start_leptos;
    
    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    start_leptos(addr.port()).await;
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
