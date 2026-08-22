#[cfg(target_arch = "wasm32")]
use crate::browser_api_client::fetch_file_info;
use crate::components::context::{
    ActivePaneContext, LogContentFocusContext, LogFileContext, SearchCommandContext,
    SearchUiContext, SelectionContext, SelectionSource,
};
use crate::components::filter_pane::FilterPane;
use crate::components::main_pane::MainPane;
use crate::components::search_panel::SearchPanel;
use crate::components::{AppBar, VisualRules};
#[cfg(target_arch = "wasm32")]
use leptos::ev::{keydown, KeyboardEvent};
use leptos::html;
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use leptos::wasm_bindgen::JsCast;
use leptos::{component, view, IntoView};
use leptos_router::hooks::use_params_map;
#[cfg(target_arch = "wasm32")]
use leptos_use::use_event_listener;

#[cfg(any(target_arch = "wasm32", test))]
fn is_editable_element(tag_name: &str, content_editable: Option<&str>) -> bool {
    matches!(tag_name, "INPUT" | "TEXTAREA" | "SELECT")
        || content_editable
            .map(|value| value.eq_ignore_ascii_case("true") || value.is_empty())
            .unwrap_or(false)
}

#[cfg(any(target_arch = "wasm32", test))]
fn should_handle_viewer_shortcut(
    is_editable_target: bool,
    is_visual_rules_target: bool,
    allowed_in_editable_target: bool,
) -> bool {
    !is_visual_rules_target && (allowed_in_editable_target || !is_editable_target)
}

#[cfg(target_arch = "wasm32")]
fn is_editable_target(target: Option<&leptos::web_sys::HtmlElement>) -> bool {
    target
        .map(|element| {
            is_editable_element(
                &element.tag_name(),
                element.get_attribute("contenteditable").as_deref(),
            ) || element.is_content_editable()
        })
        .unwrap_or(false)
}

#[cfg(target_arch = "wasm32")]
fn is_visual_rules_target(target: Option<&leptos::web_sys::HtmlElement>) -> bool {
    target
        .and_then(|element| {
            element
                .closest(r#"[data-viewer-shortcuts="ignore"]"#)
                .ok()
                .flatten()
        })
        .is_some()
}

#[cfg_attr(not(any(target_arch = "wasm32", test)), allow(dead_code))]
fn app_bar_path(file_info: Option<&logmancer_core::FileInfo>, fallback: &str) -> String {
    file_info
        .map(|file_info| file_info.path.clone())
        .unwrap_or_else(|| fallback.to_string())
}

fn next_refresh_generation(current: u64) -> u64 {
    current.wrapping_add(1)
}

#[component]
pub fn LogView() -> impl IntoView {
    let file_id = Memo::new(move |_| use_params_map().get().get("id").unwrap_or_default());
    let (follow, set_follow) = signal(false);
    let (tail, set_tail) = signal(false);
    let (selected_original_line, set_selected_original_line) = signal(None::<usize>);
    let (selected_line_source, set_selected_line_source) = signal(SelectionSource::Main);
    let (active_pane, set_active_pane) = signal(SelectionSource::Main);
    let (filter_height_percent, set_filter_height_percent) = signal(30.0_f64);
    let (is_resizing, set_is_resizing) = signal(false);
    let (search_panel_visible, set_search_panel_visible) = signal(false);
    let (search_query, set_search_query) = signal(String::new());
    let (search_status, set_search_status) = signal(String::new());
    let (search_focus_request, request_search_focus) = signal(0_u64);
    let (search_close_request, request_search_close) = signal(0_u64);
    let (search_submit_request, request_search_submit) = signal(0_u64);
    let (search_clear_request, request_search_clear) = signal(0_u64);
    let (search_next_request, request_search_next) = signal(0_u64);
    let (search_previous_request, request_search_previous) = signal(0_u64);
    let (search_navigation_in_flight, set_search_navigation_in_flight) = signal(false);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (&request_search_next, &request_search_previous);
    let (log_content_focus_request, request_log_content_focus) = signal(0_u64);
    let log_view_ref: NodeRef<html::Div> = NodeRef::new();
    let (visual_rules_open, set_visual_rules_open) = signal(false);
    let visual_rules_button_ref: NodeRef<html::Button> = NodeRef::new();
    let (visual_rules_refresh_generation, set_visual_rules_refresh_generation) = signal(0_u64);
    let (file_path, set_file_path) = signal(file_id.get_untracked());
    #[cfg(not(target_arch = "wasm32"))]
    let _ = set_file_path;

    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        let current_file_id = file_id.get();
        set_file_path.set(current_file_id.clone());
        leptos::task::spawn_local(async move {
            if let Ok(info) = fetch_file_info(current_file_id).await {
                set_file_path.set(app_bar_path(Some(&info), ""));
            }
        });
    });

    let focus_main_content = move || {
        set_active_pane.set(SelectionSource::Main);
        request_log_content_focus.update(|request| *request = request.saturating_add(1));
    };

    #[cfg(target_arch = "wasm32")]
    let open_search_panel = move || {
        set_search_panel_visible.set(true);
        request_search_focus.update(|request| *request = request.saturating_add(1));
    };

    let close_search_panel = move || {
        set_search_panel_visible.set(false);
        focus_main_content();

        if let Some(log_view) = log_view_ref.get() {
            request_animation_frame(move || {
                _ = log_view.focus();
            });
        }
    };

    #[cfg(target_arch = "wasm32")]
    let _search_shortcut_cleanup =
        use_event_listener(web_sys::window(), keydown, move |ev: KeyboardEvent| {
            let target = ev
                .target()
                .and_then(|target| target.dyn_into::<leptos::web_sys::HtmlElement>().ok());
            let is_editable_target = is_editable_target(target.as_ref());
            let is_visual_rules_target = is_visual_rules_target(target.as_ref());
            let opens_with_slash = ev.key() == "/"
                && !ev.ctrl_key()
                && !ev.meta_key()
                && !ev.alt_key()
                && should_handle_viewer_shortcut(is_editable_target, is_visual_rules_target, false);
            let opens_with_find = (ev.ctrl_key() || ev.meta_key())
                && ev.key().eq_ignore_ascii_case("f")
                && should_handle_viewer_shortcut(is_editable_target, is_visual_rules_target, true);
            let search_next = ev.key() == "n"
                && !ev.ctrl_key()
                && !ev.meta_key()
                && !ev.alt_key()
                && should_handle_viewer_shortcut(is_editable_target, is_visual_rules_target, false);
            let search_previous = ev.key() == "N"
                && !ev.ctrl_key()
                && !ev.meta_key()
                && !ev.alt_key()
                && should_handle_viewer_shortcut(is_editable_target, is_visual_rules_target, false);

            if is_visual_rules_target {
                return;
            }

            if ev.key() == "Escape" {
                if search_panel_visible.get_untracked() {
                    request_search_close.update(|request| *request = request.saturating_add(1));
                }
                return;
            }

            if opens_with_find || opens_with_slash {
                ev.prevent_default();
                open_search_panel();
                return;
            }

            if search_next {
                ev.prevent_default();
                request_search_next.update(|request| *request = request.saturating_add(1));
                return;
            }

            if search_previous {
                ev.prevent_default();
                request_search_previous.update(|request| *request = request.saturating_add(1));
            }
        });

    let update_split = move |event: leptos::ev::PointerEvent| {
        if !is_resizing.get() {
            return;
        }

        let Some(container) = log_view_ref.get() else {
            return;
        };

        let rect = container.get_bounding_client_rect();
        let container_height = rect.height();
        if container_height <= 0.0 {
            return;
        }

        let pointer_y = (event.client_y() as f64 - rect.top()).clamp(0.0, container_height);
        let raw_filter_percent = ((container_height - pointer_y) / container_height) * 100.0;
        let clamped_filter_percent = raw_filter_percent.clamp(15.0, 70.0);
        set_filter_height_percent.set(clamped_filter_percent);
    };

    provide_context(LogFileContext {
        file_id,
        tail,
        set_tail,
        follow,
        set_follow,
    });

    provide_context(SelectionContext {
        selected_original_line,
        set_selected_original_line,
        selected_line_source,
        set_selected_line_source,
    });

    provide_context(ActivePaneContext {
        active_pane,
        set_active_pane,
    });

    provide_context(SearchUiContext {
        visible: search_panel_visible,
        query: search_query,
        set_query: set_search_query,
        status: search_status,
        set_status: set_search_status,
        focus_request: search_focus_request,
        request_focus: request_search_focus,
        request_close: request_search_close,
    });

    provide_context(SearchCommandContext {
        submit_request: search_submit_request,
        request_submit: request_search_submit,
        clear_request: search_clear_request,
        request_clear: request_search_clear,
        next_request: search_next_request,
        previous_request: search_previous_request,
        navigation_in_flight: search_navigation_in_flight,
        set_navigation_in_flight: set_search_navigation_in_flight,
    });

    provide_context(LogContentFocusContext {
        focus_request: log_content_focus_request,
        request_focus: request_log_content_focus,
    });

    Effect::new(move || {
        let request = search_close_request.get();
        if request == 0 || !search_panel_visible.get_untracked() {
            return;
        }

        close_search_panel();
    });

    view! {
        <div
            node_ref=log_view_ref
            class=move || {
                if is_resizing.get() {
                    "log-view resizing"
                } else {
                    "log-view"
                }
            }
            on:pointermove=update_split
            on:pointerup=move |_| set_is_resizing.set(false)
            on:pointerleave=move |_| set_is_resizing.set(false)
            tabindex="0"
        >
            <AppBar
                path=file_path
                open_visual_rules=Callback::new(move |_| set_visual_rules_open.set(true))
                visual_rules_button_ref=visual_rules_button_ref
            />
            <div
                class=move || {
                    if active_pane.get() == SelectionSource::Main {
                        "main-pane-container pane-active"
                    } else {
                        "main-pane-container pane-inactive"
                    }
                }
                style=move || {
                    let main_height_percent = 100.0 - filter_height_percent.get();
                    format!("flex: {main_height_percent} {main_height_percent} 0;")
                }
            >
                <MainPane refresh_generation=visual_rules_refresh_generation />
            </div>
            <div class="divider" on:pointerdown=move |_| set_is_resizing.set(true)></div>
            <div
                class=move || {
                    if active_pane.get() == SelectionSource::Filter {
                        "filter-pane-container pane-active"
                    } else {
                        "filter-pane-container pane-inactive"
                    }
                }
                style=move || {
                    let filter_height_percent = filter_height_percent.get();
                    format!("flex: {filter_height_percent} {filter_height_percent} 0;")
                }
            >
                <FilterPane refresh_generation=visual_rules_refresh_generation />
            </div>
            <SearchPanel />
            <VisualRules
                open=visual_rules_open
                set_open=set_visual_rules_open
                invoker_ref=visual_rules_button_ref
                on_saved=Callback::new(move |_| {
                    set_visual_rules_refresh_generation.update(|generation| {
                        *generation = next_refresh_generation(*generation);
                    });
                })
            />
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::{
        app_bar_path, is_editable_element, next_refresh_generation, should_handle_viewer_shortcut,
    };
    use logmancer_core::FileInfo;

    #[test]
    fn editable_detection_matches_form_controls_and_contenteditable() {
        assert!(is_editable_element("INPUT", None));
        assert!(is_editable_element("TEXTAREA", None));
        assert!(is_editable_element("DIV", Some("true")));
        assert!(!is_editable_element("DIV", None));
    }

    #[test]
    fn viewer_shortcuts_ignore_the_visual_rules_surface_and_editable_targets() {
        assert!(!should_handle_viewer_shortcut(false, true, false));
        assert!(!should_handle_viewer_shortcut(false, true, true));
        assert!(should_handle_viewer_shortcut(false, false, false));
        assert!(should_handle_viewer_shortcut(false, false, true));
        assert!(!should_handle_viewer_shortcut(true, false, false));
        assert!(should_handle_viewer_shortcut(true, false, true));
    }

    #[test]
    fn app_bar_path_uses_the_open_file_path_when_file_info_is_available() {
        let file_info = FileInfo {
            path: "/var/log/service.log".to_string(),
            total_lines: 12,
            indexing_progress: 1.0,
        };

        assert_eq!(
            app_bar_path(Some(&file_info), "file-id"),
            "/var/log/service.log"
        );
        assert_eq!(app_bar_path(None, "file-id"), "file-id");
    }

    #[test]
    fn page_refresh_generation_advances_for_each_accepted_save() {
        assert_eq!(next_refresh_generation(0), 1);
        assert_eq!(next_refresh_generation(1), 2);
        assert_eq!(next_refresh_generation(u64::MAX), 0);
    }
}
