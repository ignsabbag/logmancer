#![recursion_limit = "256"]

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use leptos::prelude::*;
    use logmancer_web::init_backend_logging;
    use logmancer_web::{start_leptos_with_options, WebServerOptions};
    use std::process;
    use tracing::info;

    init_backend_logging();

    let conf = get_configuration(None).unwrap();
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    let options = match WebServerOptions::from_env(&arguments, conf.leptos_options.site_addr.port())
    {
        Ok(options) => options,
        Err(error) if error.is_help() => {
            println!("{error}");
            return;
        }
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    };
    info!(
        "Launching logmancer-web SSR on {} with LEPTOS_SITE_ROOT={:?} LEPTOS_OUTPUT_NAME={:?}",
        options.bind_addr,
        std::env::var("LEPTOS_SITE_ROOT").ok(),
        std::env::var("LEPTOS_OUTPUT_NAME").ok()
    );
    if let Err(error) = start_leptos_with_options(options).await {
        eprintln!("{error}");
        process::exit(2);
    }
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
