#[cfg(any(target_arch = "wasm32", test))]
const SCROLL_TRACE_FLAGS: &[&str] = &["scroll_trace", "scrollTrace", "debug_scroll"];
#[cfg(any(target_arch = "wasm32", test))]
const ENABLED_VALUES: &[&str] = &["", "1", "true", "yes", "on"];

macro_rules! scroll_trace {
    ($enabled:expr, $($arg:tt)*) => {
        if $enabled {
            leptos::logging::log!($($arg)*);
        }
    };
}

pub(crate) use scroll_trace;

pub fn scroll_trace_enabled() -> bool {
    scroll_trace_query_enabled()
}

#[cfg(target_arch = "wasm32")]
fn scroll_trace_query_enabled() -> bool {
    web_sys::window()
        .and_then(|window| window.location().search().ok())
        .is_some_and(|search| query_flag_enabled(&search, SCROLL_TRACE_FLAGS))
}

#[cfg(not(target_arch = "wasm32"))]
fn scroll_trace_query_enabled() -> bool {
    false
}

#[cfg(any(target_arch = "wasm32", test))]
fn query_flag_enabled(search: &str, flags: &[&str]) -> bool {
    search.trim_start_matches('?').split('&').any(|pair| {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or_default();
        let value = parts.next().unwrap_or("true");

        flags.contains(&key) && ENABLED_VALUES.contains(&value)
    })
}

#[cfg(test)]
mod tests {
    use super::query_flag_enabled;

    #[test]
    fn query_flag_accepts_enabled_values() {
        let flags = &["scroll_trace"];

        assert!(query_flag_enabled("?scroll_trace", flags));
        assert!(query_flag_enabled("?scroll_trace=1", flags));
        assert!(query_flag_enabled("?scroll_trace=true", flags));
        assert!(query_flag_enabled("?foo=bar&scroll_trace=on", flags));
    }

    #[test]
    fn query_flag_rejects_missing_or_disabled_values() {
        let flags = &["scroll_trace"];

        assert!(!query_flag_enabled("?foo=bar", flags));
        assert!(!query_flag_enabled("?scroll_trace=0", flags));
        assert!(!query_flag_enabled("?scroll_trace=false", flags));
    }
}
