pub mod commons;

#[cfg(feature = "ssr")]
mod config;

#[cfg(feature = "ssr")]
pub use config::api_routes;

#[cfg(feature = "ssr")]
pub mod open_server_file;

#[cfg(feature = "ssr")]
pub mod read_page;