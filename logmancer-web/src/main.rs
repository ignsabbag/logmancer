#![recursion_limit = "256"]

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use leptos::prelude::*;
    use logmancer_web::runtime_parameter::{
        resolve_runtime_parameter_source, LEPTOS_OUTPUT_NAME_SOURCE_ENV,
    };
    use logmancer_web::{start_leptos_with_options, WebServerOptions};
    use std::process;
    use tracing::info;

    if let Err(error) = logmancer_web::initialize_web_file_logging() {
        eprintln!("{error}");
        process::exit(1);
    }

    let conf = get_configuration(None).unwrap();
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    let resolved_options = match WebServerOptions::resolve_from_env(
        &arguments,
        conf.leptos_options.site_addr.port(),
    ) {
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
        bind_addr_source = %resolved_options.bind_addr_source,
        file_root_source = %resolved_options.file_root_source,
        output_name_source = %resolve_runtime_parameter_source(
            std::env::var_os("LEPTOS_OUTPUT_NAME").as_deref(),
            std::env::var_os(LEPTOS_OUTPUT_NAME_SOURCE_ENV).as_deref(),
        ),
        "Resolved logmancer-web runtime parameters"
    );
    if let Err(error) = start_leptos_with_options(resolved_options.options).await {
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
