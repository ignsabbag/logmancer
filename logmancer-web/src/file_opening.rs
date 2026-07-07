#[cfg(feature = "ssr")]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "ssr")]
static DESKTOP_SSR_RUNTIME: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileOpeningRuntime {
    Web,
    Desktop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileOpeningCapabilities {
    pub browser_upload: bool,
    pub server_browser: bool,
    pub desktop_native_open: bool,
}

pub fn resolve_file_opening_capabilities(runtime: FileOpeningRuntime) -> FileOpeningCapabilities {
    match runtime {
        FileOpeningRuntime::Web => FileOpeningCapabilities {
            browser_upload: true,
            server_browser: true,
            desktop_native_open: false,
        },
        FileOpeningRuntime::Desktop => FileOpeningCapabilities {
            browser_upload: false,
            server_browser: false,
            desktop_native_open: true,
        },
    }
}

pub fn initial_file_opening_capabilities() -> FileOpeningCapabilities {
    resolve_file_opening_capabilities(initial_file_opening_runtime())
}

#[cfg(feature = "ssr")]
pub fn enable_desktop_ssr_runtime() {
    DESKTOP_SSR_RUNTIME.store(true, Ordering::Relaxed);
    tracing::info!("Desktop SSR runtime flag enabled");
}

#[cfg(all(feature = "ssr", not(feature = "hydrate")))]
fn initial_file_opening_runtime() -> FileOpeningRuntime {
    let is_desktop = DESKTOP_SSR_RUNTIME.load(Ordering::Relaxed);
    tracing::info!(
        desktop_ssr_runtime_enabled = is_desktop,
        "Resolving initial file opening runtime for SSR"
    );

    if is_desktop {
        FileOpeningRuntime::Desktop
    } else {
        FileOpeningRuntime::Web
    }
}

#[cfg(feature = "hydrate")]
fn initial_file_opening_runtime() -> FileOpeningRuntime {
    detect_file_opening_runtime()
}

#[cfg(not(any(feature = "ssr", feature = "hydrate")))]
fn initial_file_opening_runtime() -> FileOpeningRuntime {
    FileOpeningRuntime::Web
}

#[cfg(any(feature = "hydrate", test))]
fn is_desktop_runtime_marker(search: &str) -> bool {
    search
        .trim_start_matches('?')
        .split('&')
        .any(|param| param == "runtime=desktop")
}

#[cfg(feature = "hydrate")]
pub fn detect_file_opening_runtime() -> FileOpeningRuntime {
    let window = leptos::prelude::window();
    let search = window.location().search().unwrap_or_default();
    let has_desktop_marker = is_desktop_runtime_marker(&search);
    let tauri_key = wasm_bindgen::JsValue::from_str("__TAURI__");
    let has_tauri_global = js_sys::Reflect::get(window.as_ref(), &tauri_key)
        .map(|value| !value.is_undefined() && !value.is_null())
        .unwrap_or(false);

    leptos::logging::log!(
        "File opening runtime detection: search='{}' desktop_marker={} tauri_global={}",
        search,
        has_desktop_marker,
        has_tauri_global
    );

    if has_desktop_marker {
        return FileOpeningRuntime::Desktop;
    }

    if has_tauri_global {
        FileOpeningRuntime::Desktop
    } else {
        FileOpeningRuntime::Web
    }
}

#[cfg(not(feature = "hydrate"))]
pub fn detect_file_opening_runtime() -> FileOpeningRuntime {
    FileOpeningRuntime::Web
}

#[cfg(feature = "hydrate")]
pub async fn open_native_log_file() -> Result<Option<String>, String> {
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;

    leptos::logging::log!("Invoking desktop native file picker");

    let window = leptos::prelude::window();
    let tauri = js_sys::Reflect::get(window.as_ref(), &JsValue::from_str("__TAURI__"))
        .map_err(|_| "Desktop native file opening is unavailable.".to_string())?;
    let core = js_sys::Reflect::get(&tauri, &JsValue::from_str("core"))
        .map_err(|_| "Desktop native file opening is unavailable.".to_string())?;
    let invoke = js_sys::Reflect::get(&core, &JsValue::from_str("invoke"))
        .map_err(|_| "Desktop native file opening is unavailable.".to_string())?
        .dyn_into::<js_sys::Function>()
        .map_err(|_| "Desktop native file opening is unavailable.".to_string())?;
    let promise = invoke
        .call1(&core, &JsValue::from_str("open_native_log_file"))
        .map_err(|_| "Could not open the native file dialog.".to_string())?
        .dyn_into::<js_sys::Promise>()
        .map_err(|_| "Desktop native file opening returned an invalid response.".to_string())?;

    let value = JsFuture::from(promise).await.map_err(|error| {
        leptos::logging::log!("Desktop native file picker failed: {:?}", error);
        "Could not open the selected file.".to_string()
    })?;

    if value.is_null() || value.is_undefined() {
        leptos::logging::log!("Desktop native file picker cancelled");
        Ok(None)
    } else {
        let file_id = value.as_string().ok_or_else(|| {
            "Desktop native file opening returned an invalid file id.".to_string()
        })?;
        leptos::logging::log!("Desktop native file picker opened file_id={}", file_id);
        Ok(Some(file_id))
    }
}

#[cfg(not(feature = "hydrate"))]
pub async fn open_native_log_file() -> Result<Option<String>, String> {
    Err("Desktop native file opening is unavailable.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_runtime_exposes_upload_and_server_browser_only() {
        let capabilities = resolve_file_opening_capabilities(FileOpeningRuntime::Web);

        assert!(capabilities.browser_upload);
        assert!(capabilities.server_browser);
        assert!(!capabilities.desktop_native_open);
    }

    #[test]
    fn desktop_runtime_hides_upload_and_exposes_native_open() {
        let capabilities = resolve_file_opening_capabilities(FileOpeningRuntime::Desktop);

        assert!(!capabilities.browser_upload);
        assert!(!capabilities.server_browser);
        assert!(capabilities.desktop_native_open);
    }

    #[test]
    fn initial_capabilities_match_ssr_web_tree() {
        assert_eq!(
            initial_file_opening_capabilities(),
            resolve_file_opening_capabilities(FileOpeningRuntime::Web)
        );
    }

    #[test]
    fn desktop_runtime_marker_is_detected_from_query_string() {
        assert!(is_desktop_runtime_marker("?runtime=desktop"));
        assert!(is_desktop_runtime_marker("?foo=bar&runtime=desktop"));
        assert!(!is_desktop_runtime_marker("?runtime=web"));
    }
}
