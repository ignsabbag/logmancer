use crate::components::context::{
    ActivePaneContext, LogFileContext, LogViewContext, SelectionSource,
};
use crate::components::diagnostics::{scroll_trace, scroll_trace_enabled};
use leptos::context::use_context;
use leptos::ev::{KeyboardEvent, WheelEvent};
use leptos::logging::log;
use leptos::prelude::*;
use leptos::{component, html, view, IntoView};
use logmancer_core::PageResult;
use std::time::Duration;

const SCROLL_RATIO: f64 = 0.15;
const DEBOUNCE_MS: u64 = 200;
const MIN_JUMP: i32 = 2;

const ARROW_UP: &str = "ArrowUp";
const ARROW_DOWN: &str = "ArrowDown";
const PAGE_UP: &str = "PageUp";
const PAGE_DOWN: &str = "PageDown";

fn is_handled_key(key: &str) -> bool {
    matches!(
        key,
        ARROW_DOWN | ARROW_UP | PAGE_DOWN | PAGE_UP | "g" | "G" | "f" | "F"
    )
}

fn is_editable_target(tag_name: Option<&str>, content_editable: bool) -> bool {
    if content_editable {
        return true;
    }

    matches!(tag_name, Some("INPUT") | Some("TEXTAREA") | Some("SELECT"))
}

fn should_restore_focus(
    is_active_panel: bool,
    active_tag_name: Option<&str>,
    content_editable: bool,
) -> bool {
    is_active_panel && !is_editable_target(active_tag_name, content_editable)
}

fn can_auto_enable_global_follow(selection_source: SelectionSource) -> bool {
    matches!(selection_source, SelectionSource::Main)
}

fn can_mutate_global_follow_state(selection_source: SelectionSource) -> bool {
    matches!(selection_source, SelectionSource::Main)
}

#[component]
pub fn ContentLines(context: LogViewContext) -> impl IntoView {
    let LogFileContext {
        tail,
        set_tail,
        follow,
        set_follow,
        ..
    } = use_context().expect("LogFileContext not found");

    let LogViewContext {
        set_start_line,
        page_size,
        set_page_size,
        log_page,
        selected_line,
        set_selected_line,
        selection_source,
        set_selected_line_source,
        set_active_pane,
        ..
    } = context;

    let ActivePaneContext { active_pane, .. } = use_context().expect("ActivePaneContext not found");

    let select_line = move |line_number| {
        set_active_pane.set(selection_source);
        set_selected_line_source.set(selection_source);
        set_selected_line.set(Some(line_number));
    };

    let div_ref: NodeRef<html::Div> = NodeRef::new();
    let (max_width, set_max_width) = signal(0);

    let (wheel_target_line, set_wheel_target_line) = signal(None::<usize>);
    let (debounce, set_debounce) = signal(None::<TimeoutHandle>);
    let (wheel_event_id, set_wheel_event_id) = signal(0_u64);
    let (page_result, set_page_result) = signal(None::<PageResult>);
    let scroll_trace = scroll_trace_enabled();

    let is_at_end = move |result: &PageResult| {
        result.start_line.saturating_add(page_size.get()) >= result.total_lines
    };

    let update_tail = move |new_line: usize| {
        if !can_mutate_global_follow_state(selection_source) {
            return;
        }

        set_tail.update_untracked(move |current| {
            let page_result = page_result.get().unwrap();
            if page_result.start_line > new_line {
                log!("Updating tail to false");
                *current = false;
                set_follow.set(false);
            } else if new_line.saturating_add(page_size.get()) >= page_result.total_lines {
                log!("Updating tail to true");
                *current = true
            }
        });
    };

    let process_key = move |key: &str| {
        let page_result = page_result.get().unwrap();
        log!("Key {} pressed", key);
        match key {
            ARROW_UP => {
                let new_line = page_result.start_line.saturating_sub(MIN_JUMP as usize);
                update_tail(new_line);
                set_start_line.set(new_line);
            }
            ARROW_DOWN => {
                if is_at_end(&page_result) {
                    if can_auto_enable_global_follow(selection_source) {
                        set_tail.set(true);
                        set_follow.set(true);
                    }
                    return;
                }
                let new_line = page_result.start_line.saturating_add(MIN_JUMP as usize);
                update_tail(new_line);
                set_start_line.set(new_line);
            }
            PAGE_UP => {
                let new_line = page_result.start_line.saturating_sub(page_size.get());
                update_tail(new_line);
                set_start_line.set(new_line);
            }
            PAGE_DOWN => {
                if is_at_end(&page_result) {
                    if can_auto_enable_global_follow(selection_source) {
                        set_tail.set(true);
                        set_follow.set(true);
                    }
                    return;
                }
                let new_line = page_result.start_line.saturating_add(page_size.get());
                update_tail(new_line);
                set_start_line.set(new_line);
            }
            "g" => {
                update_tail(0);
                set_start_line.set(0);
            }
            "G" if can_auto_enable_global_follow(selection_source) => {
                set_tail.set(true);
            }
            "f" | "F" if can_mutate_global_follow_state(selection_source) => {
                set_follow.update(|current| *current = !current.to_owned());
            }
            _ => (),
        }
    };

    let on_key_down = move |ev: KeyboardEvent| {
        set_active_pane.set(selection_source);
        let key = ev.key();
        if is_handled_key(&key) {
            ev.prevent_default();
            process_key(&key);
        }
    };

    let on_key_up = move |ev: KeyboardEvent| {
        let key = ev.key();
        if is_handled_key(&key) {
            ev.prevent_default();
        }
    };

    let on_wheel = move |ev: WheelEvent| {
        set_active_pane.set(selection_source);
        ev.prevent_default();

        let event_id = wheel_event_id.get_untracked().saturating_add(1);
        set_wheel_event_id.set(event_id);

        let delta = ev.delta_y().abs();
        let signum = ev.delta_y().signum() as i32;
        let is_precise_scroll = ev.delta_mode() == 0;

        scroll_trace!(
            scroll_trace,
            "scroll-trace wheel event_id={} delta_y={} delta_mode={} precise={} debounce_active={}",
            event_id,
            ev.delta_y(),
            ev.delta_mode(),
            is_precise_scroll,
            debounce.get_untracked().is_some()
        );
        let lines_to_jump = if is_precise_scroll {
            let lines = delta * SCROLL_RATIO;
            MIN_JUMP.max(lines as i32)
        } else {
            MIN_JUMP.max((page_size.get() as f64 * SCROLL_RATIO) as i32)
        };
        let signed_wheel_lines = lines_to_jump * signum;
        scroll_trace!(
            scroll_trace,
            "scroll-trace wheel event_id={} lines_to_jump={} signed_wheel_lines={}",
            event_id,
            lines_to_jump,
            signed_wheel_lines
        );

        let Some(current_page_result) = page_result.get() else {
            return;
        };

        if signed_wheel_lines > 0 && is_at_end(&current_page_result) {
            if can_auto_enable_global_follow(selection_source) {
                set_tail.set(true);
                set_follow.set(true);
            }
            return;
        }

        let max_start_line = current_page_result
            .total_lines
            .saturating_sub(page_size.get_untracked());
        let base_line = wheel_target_line
            .get_untracked()
            .unwrap_or(current_page_result.start_line);
        let target_line = if signed_wheel_lines < 0 {
            base_line.saturating_sub(signed_wheel_lines.unsigned_abs() as usize)
        } else {
            base_line
                .saturating_add(signed_wheel_lines as usize)
                .min(max_start_line)
        };
        set_wheel_target_line.set(Some(target_line));
        scroll_trace!(
            scroll_trace,
            "scroll-trace wheel target event_id={} base_line={} target_line={} max_start_line={}",
            event_id,
            base_line,
            target_line,
            max_start_line
        );

        if debounce.get().is_none() {
            scroll_trace!(
                scroll_trace,
                "scroll-trace wheel schedule event_id={} current_start_line={} total_lines={} page_size={}",
                event_id,
                current_page_result.start_line,
                current_page_result.total_lines,
                page_size.get_untracked()
            );
            let handle = set_timeout_with_handle(
                move || {
                    let Some(new_line) = wheel_target_line.get() else {
                        set_debounce.set(None);
                        return;
                    };
                    let Some(current_page_result) = page_result.get() else {
                        set_debounce.set(None);
                        return;
                    };
                    scroll_trace!(
                        scroll_trace,
                        "scroll-trace wheel fire event_id={} current_start_line={} target_line={}",
                        event_id,
                        current_page_result.start_line,
                        new_line
                    );
                    if current_page_result.start_line != new_line {
                        scroll_trace!(
                            scroll_trace,
                            "scroll-trace wheel apply event_id={} start_line_before={} start_line_after={}",
                            event_id,
                            current_page_result.start_line,
                            new_line
                        );
                        if can_mutate_global_follow_state(selection_source) {
                            set_tail.update_untracked(move |current| {
                                if current_page_result.start_line > new_line {
                                    *current = false;
                                    set_follow.set(false);
                                } else if new_line + page_size.get()
                                    > current_page_result.total_lines
                                {
                                    *current = true
                                }
                            });
                        }
                        set_start_line.set(new_line);
                    }
                    set_debounce.set(None);
                },
                Duration::from_millis(DEBOUNCE_MS),
            )
            .ok();
            set_debounce.set(handle);
        }
    };

    Effect::new(move || {
        page_result.track();
        if let Some(div) = div_ref.get() {
            let max_width = max_width.get();
            request_animation_frame(move || {
                if div.scroll_width() > max_width {
                    set_max_width.set(div.scroll_width());
                    (*div)
                        .style()
                        .set_property("min-width", format!("{}px", div.scroll_width()).as_str())
                        .unwrap();
                }
            });
        }
    });

    Effect::new(move || {
        page_result.track();

        let Some(div) = div_ref.get() else {
            return;
        };

        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(document) = window.document() else {
            return;
        };

        let (tag_name, content_editable) = match document.active_element() {
            Some(active) => (
                Some(active.tag_name()),
                active
                    .get_attribute("contenteditable")
                    .map(|value| value.eq_ignore_ascii_case("true") || value.is_empty())
                    .unwrap_or(false),
            ),
            None => (None, false),
        };

        if should_restore_focus(
            active_pane.get() == selection_source,
            tag_name.as_deref(),
            content_editable,
        ) {
            _ = div.focus();
        }
    });

    view! {
        <Transition>
            { move || Suspend::new(async move {
                log_page.await.map(|page_result| {
                    set_page_result.set(Some(page_result.clone()));
                    if debounce.get_untracked().is_none() {
                        set_wheel_target_line.set(Some(page_result.start_line));
                    }
                    if tail.get() && follow.get() {
                        set_timeout(move || set_page_size.notify(), Duration::from_secs(1));
                    } else {
                        update_tail(page_result.start_line);
                    }
                    let lines = page_result.lines;
                    view! {
                        <div class="line-numbers">
                            { lines.iter().map(|line| {
                                let line_number = line.number;
                                view! {
                                    <div
                                        on:click=move |_| select_line(line_number)
                                    >
                                        <b>{line_number}</b>
                                    </div>
                                }
                            }).collect::<Vec<_>>() }
                        </div>
                        <div
                            node_ref=div_ref
                            class="text-lines" tabindex="0"
                            on:focus=move |_| set_active_pane.set(selection_source)
                            on:keydown=on_key_down on:keyup=on_key_up
                            on:wheel=on_wheel
                        >
                            { lines.into_iter().map(|line| {
                                let line_number = line.number;
                                let line_text = line.text;
                                view! {
                                    <div
                                        class:selected=move || selected_line.get() == Some(line_number)
                                        on:click=move |_| select_line(line_number)
                                    >
                                        {line_text}
                                    </div>
                                }
                            }).collect::<Vec<_>>() }
                        </div>
                    }
                })
            })}
        </Transition>
    }
}

#[cfg(test)]
mod tests {
    use super::{
        can_auto_enable_global_follow, can_mutate_global_follow_state, is_editable_target,
        is_handled_key, should_restore_focus,
    };
    use crate::components::context::SelectionSource;

    #[test]
    fn handled_keys_include_navigation_and_commands() {
        assert!(is_handled_key("ArrowUp"));
        assert!(is_handled_key("PageDown"));
        assert!(is_handled_key("g"));
        assert!(is_handled_key("G"));
        assert!(is_handled_key("f"));
        assert!(is_handled_key("F"));
    }

    #[test]
    fn handled_keys_exclude_unrelated_keys() {
        assert!(!is_handled_key("Enter"));
        assert!(!is_handled_key("a"));
    }

    #[test]
    fn editable_targets_are_detected() {
        assert!(is_editable_target(Some("INPUT"), false));
        assert!(is_editable_target(Some("TEXTAREA"), false));
        assert!(is_editable_target(Some("DIV"), true));
    }

    #[test]
    fn restore_focus_only_for_active_non_editable_target() {
        assert!(should_restore_focus(true, Some("DIV"), false));
        assert!(!should_restore_focus(false, Some("DIV"), false));
        assert!(!should_restore_focus(true, Some("INPUT"), false));
    }

    #[test]
    fn auto_follow_is_only_enabled_for_main_pane() {
        assert!(can_auto_enable_global_follow(SelectionSource::Main));
        assert!(!can_auto_enable_global_follow(SelectionSource::Filter));
    }

    #[test]
    fn global_follow_state_mutation_is_only_allowed_for_main_pane() {
        assert!(can_mutate_global_follow_state(SelectionSource::Main));
        assert!(!can_mutate_global_follow_state(SelectionSource::Filter));
    }
}
