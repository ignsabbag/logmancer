#[cfg(feature = "ssr")]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "ssr")]
static DESKTOP_SSR_RUNTIME: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileOpeningRuntime {
    Web,
    DesktopEmbedded,
    DesktopExternal,
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
        FileOpeningRuntime::DesktopEmbedded => FileOpeningCapabilities {
            browser_upload: false,
            server_browser: false,
            desktop_native_open: true,
        },
        FileOpeningRuntime::DesktopExternal => FileOpeningCapabilities {
            browser_upload: true,
            server_browser: false,
            desktop_native_open: true,
        },
    }
}

pub fn should_ignore_dom_drop(capabilities: FileOpeningCapabilities) -> bool {
    capabilities.desktop_native_open && !capabilities.browser_upload
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
        FileOpeningRuntime::DesktopEmbedded
    } else {
        FileOpeningRuntime::Web
    }
}

#[cfg(feature = "hydrate")]
fn initial_file_opening_runtime() -> FileOpeningRuntime {
    let window = leptos::prelude::window();
    let search = window.location().search().unwrap_or_default();
    let origin = window.location().origin().unwrap_or_default();

    resolve_initial_hydration_file_opening_runtime(&search, &origin)
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

#[cfg(any(feature = "hydrate", test))]
fn is_embedded_desktop_runtime_marker(search: &str) -> bool {
    search
        .trim_start_matches('?')
        .split('&')
        .any(|param| param == "runtime=desktop-embedded")
}

#[cfg(any(feature = "hydrate", test))]
fn is_external_desktop_origin(origin: &str) -> bool {
    origin == "http://localhost:3000" || origin == "http://127.0.0.1:3000"
}

#[cfg(any(feature = "hydrate", test))]
fn resolve_detected_file_opening_runtime(
    search: &str,
    origin: &str,
    has_tauri_global: bool,
) -> FileOpeningRuntime {
    let has_desktop_marker = is_desktop_runtime_marker(search);
    let has_embedded_desktop_marker = is_embedded_desktop_runtime_marker(search);

    if has_embedded_desktop_marker {
        FileOpeningRuntime::DesktopEmbedded
    } else if is_external_desktop_origin(origin) && (has_desktop_marker || has_tauri_global) {
        FileOpeningRuntime::DesktopExternal
    } else if has_desktop_marker || has_tauri_global {
        FileOpeningRuntime::DesktopEmbedded
    } else {
        FileOpeningRuntime::Web
    }
}

#[cfg(any(feature = "hydrate", test))]
fn resolve_initial_hydration_file_opening_runtime(
    search: &str,
    origin: &str,
) -> FileOpeningRuntime {
    if is_embedded_desktop_runtime_marker(search) && !is_external_desktop_origin(origin) {
        FileOpeningRuntime::DesktopEmbedded
    } else {
        FileOpeningRuntime::Web
    }
}

#[cfg(feature = "hydrate")]
pub fn detect_file_opening_runtime() -> FileOpeningRuntime {
    let window = leptos::prelude::window();
    let search = window.location().search().unwrap_or_default();
    let origin = window.location().origin().unwrap_or_default();
    let tauri_key = wasm_bindgen::JsValue::from_str("__TAURI__");
    let has_tauri_global = js_sys::Reflect::get(window.as_ref(), &tauri_key)
        .map(|value| !value.is_undefined() && !value.is_null())
        .unwrap_or(false);
    let runtime = resolve_detected_file_opening_runtime(&search, &origin, has_tauri_global);

    leptos::logging::log!(
        "File opening runtime detection: desktop_marker={} embedded_desktop_marker={} external_desktop_origin={} tauri_global={} runtime={:?}",
        is_desktop_runtime_marker(&search),
        is_embedded_desktop_runtime_marker(&search),
        is_external_desktop_origin(&origin),
        has_tauri_global,
        runtime
    );

    runtime
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
        let result = value.as_string().ok_or_else(|| {
            "Desktop native file opening returned an invalid response.".to_string()
        })?;

        match resolve_native_open_result(&result) {
            NativeOpenResult::ServerPath(path) => {
                leptos::logging::log!(
                    "Desktop native file picker returned path, opening via server API"
                );
                let file_id =
                    crate::browser_api_client::open_server_browser_file(path.to_string()).await?;
                leptos::logging::log!("Server API opened file_id={}", file_id);
                Ok(Some(file_id))
            }
            NativeOpenResult::FileId(file_id) => {
                leptos::logging::log!("Desktop native file picker opened file_id={}", file_id);
                Ok(Some(file_id.to_string()))
            }
        }
    }
}

#[cfg(any(feature = "hydrate", test))]
#[derive(Debug, Eq, PartialEq)]
enum NativeOpenResult<'a> {
    ServerPath(&'a str),
    FileId(&'a str),
}

#[cfg(any(feature = "hydrate", test))]
fn resolve_native_open_result(result: &str) -> NativeOpenResult<'_> {
    if let Some(path) = result.strip_prefix("path:") {
        NativeOpenResult::ServerPath(path)
    } else {
        NativeOpenResult::FileId(result)
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
    fn embedded_desktop_runtime_hides_upload_and_exposes_native_open() {
        let capabilities = resolve_file_opening_capabilities(FileOpeningRuntime::DesktopEmbedded);

        assert!(!capabilities.browser_upload);
        assert!(!capabilities.server_browser);
        assert!(capabilities.desktop_native_open);
    }

    #[test]
    fn external_desktop_runtime_allows_dom_upload_and_native_open() {
        let capabilities = resolve_file_opening_capabilities(FileOpeningRuntime::DesktopExternal);

        assert!(capabilities.browser_upload);
        assert!(!capabilities.server_browser);
        assert!(capabilities.desktop_native_open);
    }

    #[test]
    fn file_opening_external_desktop_dom_drop_remains_allowed_with_native_open() {
        let capabilities = resolve_file_opening_capabilities(FileOpeningRuntime::DesktopExternal);

        assert!(!should_ignore_dom_drop(capabilities));
    }

    #[test]
    fn file_opening_embedded_desktop_dom_drop_remains_ignored() {
        let capabilities = resolve_file_opening_capabilities(FileOpeningRuntime::DesktopEmbedded);

        assert!(should_ignore_dom_drop(capabilities));
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

    #[test]
    fn embedded_desktop_runtime_marker_is_detected_from_query_string() {
        assert!(is_embedded_desktop_runtime_marker(
            "?runtime=desktop-embedded"
        ));
        assert!(is_embedded_desktop_runtime_marker(
            "?foo=bar&runtime=desktop-embedded"
        ));
        assert!(!is_embedded_desktop_runtime_marker("?runtime=desktop"));
    }

    #[test]
    fn initial_hydration_external_desktop_dev_matches_web_ssr_tree() {
        assert_eq!(
            resolve_initial_hydration_file_opening_runtime(
                "?runtime=desktop",
                "http://localhost:3000"
            ),
            FileOpeningRuntime::Web
        );
    }

    #[test]
    fn initial_hydration_embedded_desktop_matches_embedded_ssr_tree() {
        assert_eq!(
            resolve_initial_hydration_file_opening_runtime(
                "?runtime=desktop-embedded",
                "http://127.0.0.1:43123"
            ),
            FileOpeningRuntime::DesktopEmbedded
        );
    }

    #[test]
    fn initial_hydration_normal_web_matches_web_ssr_tree() {
        assert_eq!(
            resolve_initial_hydration_file_opening_runtime("", "http://localhost:3000"),
            FileOpeningRuntime::Web
        );
    }

    #[test]
    fn desktop_marker_on_external_dev_origin_detects_external_desktop() {
        assert_eq!(
            resolve_detected_file_opening_runtime(
                "?runtime=desktop",
                "http://localhost:3000",
                true
            ),
            FileOpeningRuntime::DesktopExternal
        );
    }

    #[test]
    fn desktop_marker_on_external_dev_origin_detects_external_desktop_before_tauri_global() {
        assert_eq!(
            resolve_detected_file_opening_runtime(
                "?runtime=desktop",
                "http://localhost:3000",
                false
            ),
            FileOpeningRuntime::DesktopExternal
        );
    }

    #[test]
    fn tauri_global_on_external_dev_origin_detects_external_desktop_without_marker() {
        assert_eq!(
            resolve_detected_file_opening_runtime("", "http://localhost:3000", true),
            FileOpeningRuntime::DesktopExternal
        );
    }

    #[test]
    fn external_dev_origin_without_tauri_global_or_marker_stays_web() {
        assert_eq!(
            resolve_detected_file_opening_runtime("", "http://localhost:3000", false),
            FileOpeningRuntime::Web
        );
    }

    #[test]
    fn desktop_marker_on_embedded_origin_detects_embedded_desktop() {
        assert_eq!(
            resolve_detected_file_opening_runtime(
                "?runtime=desktop",
                "http://127.0.0.1:43123",
                true
            ),
            FileOpeningRuntime::DesktopEmbedded
        );
    }

    #[test]
    fn embedded_desktop_marker_detects_embedded_desktop_after_hydration() {
        assert_eq!(
            resolve_detected_file_opening_runtime(
                "?runtime=desktop-embedded",
                "http://127.0.0.1:43123",
                true
            ),
            FileOpeningRuntime::DesktopEmbedded
        );
    }

    #[test]
    fn file_opening_native_picker_path_result_uses_server_api_contract() {
        assert_eq!(
            resolve_native_open_result("path:/var/log/system.log"),
            NativeOpenResult::ServerPath("/var/log/system.log")
        );
    }

    #[test]
    fn file_opening_native_picker_file_id_result_uses_route_contract() {
        assert_eq!(
            resolve_native_open_result("file-123"),
            NativeOpenResult::FileId("file-123")
        );
    }
}
