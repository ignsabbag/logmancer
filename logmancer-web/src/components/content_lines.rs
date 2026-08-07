use crate::components::context::{
    ActivePaneContext, LogContentFocusContext, LogFileContext, LogViewContext, SearchUiContext,
    SelectionSource,
};
use crate::components::diagnostics::{scroll_trace, scroll_trace_enabled};
use crate::components::layout::{
    SCROLL_LINE_JUMP, WHEEL_SCROLL_MAX_LINE_JUMP, WHEEL_SCROLL_PIXELS_PER_LINE_STEP,
};
use crate::components::line_decorations::{
    search_decorations_by_line, split_line_segments, DecorationKind, LineDecoration,
};
use crate::components::search_status::format_page_search_status;
use leptos::context::use_context;
use leptos::ev::{KeyboardEvent, WheelEvent};
use leptos::logging::log;
use leptos::prelude::*;
use leptos::{component, html, view, IntoView};
use logmancer_core::{LineStyleIntent, PageResult, VisualColor};
use std::collections::HashMap;
use std::time::Duration;

const DEBOUNCE_MS: u64 = 200;

const ARROW_UP: &str = "ArrowUp";
const ARROW_DOWN: &str = "ArrowDown";
const PAGE_UP: &str = "PageUp";
const PAGE_DOWN: &str = "PageDown";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TailEndComparison {
    Inclusive,
    Strict,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TailNavigationUpdate {
    DisableTailAndFollow,
    EnableTail,
    NoChange,
}

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

fn is_at_end(start_line: usize, page_size: usize, total_lines: usize) -> bool {
    start_line.saturating_add(page_size) >= total_lines
}

fn keyboard_target_line(key: &str, start_line: usize, page_size: usize) -> Option<usize> {
    match key {
        ARROW_UP => Some(start_line.saturating_sub(SCROLL_LINE_JUMP)),
        ARROW_DOWN => Some(start_line.saturating_add(SCROLL_LINE_JUMP)),
        PAGE_UP => Some(start_line.saturating_sub(page_size)),
        PAGE_DOWN => Some(start_line.saturating_add(page_size)),
        "g" => Some(0),
        _ => None,
    }
}

fn wheel_target_line(base_line: usize, signed_wheel_lines: i32, max_start_line: usize) -> usize {
    if signed_wheel_lines < 0 {
        base_line.saturating_sub(signed_wheel_lines.unsigned_abs() as usize)
    } else {
        base_line
            .saturating_add(signed_wheel_lines as usize)
            .min(max_start_line)
    }
}

fn should_handle_focus_request(
    request: u64,
    last_handled_request: u64,
    active_pane: SelectionSource,
    selection_source: SelectionSource,
) -> bool {
    request != 0
        && request != last_handled_request
        && active_pane == selection_source
        && selection_source == SelectionSource::Main
}

fn tail_update_for_navigation(
    current_start_line: usize,
    new_line: usize,
    page_size: usize,
    total_lines: usize,
    end_comparison: TailEndComparison,
) -> TailNavigationUpdate {
    if current_start_line > new_line {
        return TailNavigationUpdate::DisableTailAndFollow;
    }

    let reaches_end = match end_comparison {
        TailEndComparison::Inclusive => new_line.saturating_add(page_size) >= total_lines,
        TailEndComparison::Strict => new_line.saturating_add(page_size) > total_lines,
    };

    if reaches_end {
        TailNavigationUpdate::EnableTail
    } else {
        TailNavigationUpdate::NoChange
    }
}

fn wheel_lines_to_jump(delta: f64, is_precise_scroll: bool) -> i32 {
    if delta <= 0.0 {
        return 0;
    }

    if is_precise_scroll {
        ((delta / WHEEL_SCROLL_PIXELS_PER_LINE_STEP).ceil() as i32)
            .clamp(1, WHEEL_SCROLL_MAX_LINE_JUMP)
    } else {
        WHEEL_SCROLL_MAX_LINE_JUMP
    }
}

fn search_segment_class(kind: DecorationKind) -> &'static str {
    match kind {
        DecorationKind::SearchMatch => "search-match",
        DecorationKind::SearchCurrent => "search-match search-match-current",
    }
}

fn line_decorations_for_row(
    decorations_by_line: &HashMap<usize, Vec<LineDecoration>>,
    line_number: usize,
) -> Vec<LineDecoration> {
    decorations_by_line
        .get(&line_number)
        .cloned()
        .unwrap_or_default()
}

fn visual_color_css(token: &VisualColor) -> Option<&'static str> {
    match token.0.as_str() {
        "error-foreground" => Some("#991b1b"),
        "error-background" => Some("#fee2e2"),
        "warning-foreground" => Some("#92400e"),
        "warning-background" => Some("#fef3c7"),
        _ => None,
    }
}

fn line_style_css_variables(style: Option<&LineStyleIntent>) -> Option<String> {
    let style = style?;
    let mut declarations = Vec::with_capacity(2);

    if let Some(color) = style.foreground.as_ref().and_then(visual_color_css) {
        declarations.push(format!("--log-line-foreground: {color}"));
    }
    if let Some(color) = style.background.as_ref().and_then(visual_color_css) {
        declarations.push(format!("--log-line-background: {color}"));
    }

    (!declarations.is_empty()).then(|| declarations.join("; "))
}

#[component]
fn DecoratedLineText(line_text: String, decorations: Vec<LineDecoration>) -> impl IntoView {
    let segments = split_line_segments(&line_text, &decorations);

    view! {
        {segments.into_iter().map(|segment| {
            if let Some(kind) = segment.kind {
                view! { <mark class=search_segment_class(kind)>{segment.text.to_string()}</mark> }.into_any()
            } else {
                view! { <span>{segment.text.to_string()}</span> }.into_any()
            }
        }).collect_view()}
    }
}

#[component]
fn LogLineRow(
    line_number: usize,
    line_text: String,
    line_style: Option<LineStyleIntent>,
    decorations: Vec<LineDecoration>,
    selected_line: ReadSignal<Option<usize>>,
    select_line: Callback<usize>,
) -> impl IntoView {
    let visual_style = line_style_css_variables(line_style.as_ref());
    let has_visual_style = visual_style.is_some();

    view! {
        <div
            class:selected=move || selected_line.get() == Some(line_number)
            class:visual-rule-line=has_visual_style
            style=visual_style
            on:click=move |_| select_line.run(line_number)
        >
            <DecoratedLineText line_text=line_text decorations=decorations />
        </div>
    }
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
    let LogContentFocusContext { focus_request, .. } =
        use_context().expect("LogContentFocusContext not found");
    let SearchUiContext {
        set_status: set_search_status,
        ..
    } = use_context().expect("SearchUiContext not found");

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
    let (last_handled_focus_request, set_last_handled_focus_request) = signal(0_u64);
    let scroll_trace = scroll_trace_enabled();

    let update_tail = move |new_line: usize| {
        if !can_mutate_global_follow_state(selection_source) {
            return;
        }

        set_tail.update_untracked(move |current| {
            let page_result = page_result.get().unwrap();
            match tail_update_for_navigation(
                page_result.start_line,
                new_line,
                page_size.get(),
                page_result.total_lines,
                TailEndComparison::Inclusive,
            ) {
                TailNavigationUpdate::DisableTailAndFollow => {
                    log!("Updating tail to false");
                    *current = false;
                    set_follow.set(false);
                }
                TailNavigationUpdate::EnableTail => {
                    log!("Updating tail to true");
                    *current = true
                }
                TailNavigationUpdate::NoChange => {}
            }
        });
    };

    let process_key = move |key: &str| {
        let page_result = page_result.get().unwrap();
        log!("Key {} pressed", key);
        match key {
            ARROW_DOWN | PAGE_DOWN
                if is_at_end(
                    page_result.start_line,
                    page_size.get(),
                    page_result.total_lines,
                ) && can_auto_enable_global_follow(selection_source) =>
            {
                set_tail.set(true);
                set_follow.set(true);
            }
            ARROW_UP | ARROW_DOWN | PAGE_UP | PAGE_DOWN | "g" => {
                if let Some(new_line) =
                    keyboard_target_line(key, page_result.start_line, page_size.get())
                {
                    update_tail(new_line);
                    set_start_line.set(new_line);
                }
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
        if (ev.ctrl_key() || ev.meta_key()) && key.eq_ignore_ascii_case("f") {
            return;
        }
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
        let lines_to_jump = wheel_lines_to_jump(delta, is_precise_scroll);
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

        if signed_wheel_lines > 0
            && is_at_end(
                current_page_result.start_line,
                page_size.get(),
                current_page_result.total_lines,
            )
        {
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
        let target_line = crate::components::content_lines::wheel_target_line(
            base_line,
            signed_wheel_lines,
            max_start_line,
        );
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
                                match tail_update_for_navigation(
                                    current_page_result.start_line,
                                    new_line,
                                    page_size.get(),
                                    current_page_result.total_lines,
                                    TailEndComparison::Strict,
                                ) {
                                    TailNavigationUpdate::DisableTailAndFollow => {
                                        *current = false;
                                        set_follow.set(false);
                                    }
                                    TailNavigationUpdate::EnableTail => *current = true,
                                    TailNavigationUpdate::NoChange => {}
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

    Effect::new(move || {
        let request = focus_request.get();
        if !should_handle_focus_request(
            request,
            last_handled_focus_request.get_untracked(),
            active_pane.get_untracked(),
            selection_source,
        ) {
            return;
        }

        set_last_handled_focus_request.set(request);
        if let Some(div) = div_ref.get() {
            request_animation_frame(move || {
                _ = div.focus();
            });
        }
    });

    view! {
        <Transition>
            { move || Suspend::new(async move {
                log_page.await.map(|page_result| {
                    set_search_status.set(format_page_search_status(&page_result));
                    set_page_result.set(Some(page_result.clone()));
                    if debounce.get_untracked().is_none() {
                        set_wheel_target_line.set(Some(page_result.start_line));
                    }
                    if tail.get() && follow.get() {
                        set_timeout(move || set_page_size.notify(), Duration::from_secs(1));
                    } else {
                        update_tail(page_result.start_line);
                    }
                    let search = page_result.search;
                    let lines = page_result.lines;
                    let decorations_by_line = search
                        .as_ref()
                        .map(search_decorations_by_line)
                        .unwrap_or_default();
                    let select_line_callback = Callback::new(select_line);
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
                                let line_style = line.style;
                                let decorations = line_decorations_for_row(&decorations_by_line, line_number);
                                view! {
                                    <LogLineRow
                                        line_number=line_number
                                        line_text=line_text
                                        line_style=line_style
                                        decorations=decorations
                                        selected_line=selected_line
                                        select_line=select_line_callback
                                    />
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
    use std::collections::HashMap;

    use super::{
        can_auto_enable_global_follow, can_mutate_global_follow_state, is_at_end,
        is_editable_target, is_handled_key, keyboard_target_line, line_decorations_for_row,
        line_style_css_variables, search_segment_class, should_handle_focus_request,
        should_restore_focus, tail_update_for_navigation, wheel_lines_to_jump, wheel_target_line,
        TailEndComparison, TailNavigationUpdate, ARROW_DOWN, ARROW_UP, PAGE_DOWN, PAGE_UP,
    };
    use crate::components::context::SelectionSource;
    use crate::components::line_decorations::{DecorationKind, LineDecoration};
    use logmancer_core::{LineStyleIntent, VisualColor};

    fn decoration(start: usize, end: usize, kind: DecorationKind) -> LineDecoration {
        LineDecoration { start, end, kind }
    }

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

    #[test]
    fn search_segment_class_preserves_existing_dom_classes() {
        assert_eq!(
            search_segment_class(DecorationKind::SearchMatch),
            "search-match"
        );
        assert_eq!(
            search_segment_class(DecorationKind::SearchCurrent),
            "search-match search-match-current"
        );
    }

    #[test]
    fn row_decorations_are_cloned_for_original_line_number() {
        let mut decorations_by_line = HashMap::new();
        decorations_by_line.insert(42, vec![decoration(0, 3, DecorationKind::SearchMatch)]);

        let decorations = line_decorations_for_row(&decorations_by_line, 42);

        assert_eq!(
            decorations,
            vec![decoration(0, 3, DecorationKind::SearchMatch)]
        );
    }

    #[test]
    fn row_decorations_default_to_empty_for_unmatched_original_line_number() {
        let mut decorations_by_line = HashMap::new();
        decorations_by_line.insert(7, vec![decoration(0, 3, DecorationKind::SearchCurrent)]);

        let decorations = line_decorations_for_row(&decorations_by_line, 8);

        assert_eq!(decorations, Vec::<LineDecoration>::new());
    }

    #[test]
    fn known_visual_tokens_map_to_closed_css_variables() {
        let style = LineStyleIntent {
            foreground: Some(VisualColor("error-foreground".to_string())),
            background: Some(VisualColor("error-background".to_string())),
        };

        assert_eq!(
            line_style_css_variables(Some(&style)),
            Some("--log-line-foreground: #991b1b; --log-line-background: #fee2e2".to_string())
        );
    }

    #[test]
    fn absent_or_unknown_visual_tokens_leave_row_unstyled() {
        let style = LineStyleIntent {
            foreground: Some(VisualColor("hotpink; background: url(bad)".to_string())),
            background: None,
        };

        assert_eq!(line_style_css_variables(None), None);
        assert_eq!(line_style_css_variables(Some(&style)), None);
    }

    #[test]
    fn known_visual_tokens_render_without_forwarding_unknown_tokens() {
        let style = LineStyleIntent {
            foreground: Some(VisualColor("unknown".to_string())),
            background: Some(VisualColor("warning-background".to_string())),
        };

        assert_eq!(
            line_style_css_variables(Some(&style)),
            Some("--log-line-background: #fef3c7".to_string())
        );
    }

    #[test]
    fn precise_wheel_scroll_is_capped_to_three_lines() {
        assert_eq!(wheel_lines_to_jump(1.0, true), 1);
        assert_eq!(wheel_lines_to_jump(80.0, true), 2);
        assert_eq!(wheel_lines_to_jump(240.0, true), 3);
    }

    #[test]
    fn non_precise_wheel_scroll_uses_three_lines() {
        assert_eq!(wheel_lines_to_jump(1.0, false), 3);
    }

    #[test]
    fn page_end_detection_uses_inclusive_comparison() {
        assert!(is_at_end(90, 10, 100));
        assert!(is_at_end(100, 10, 100));
        assert!(!is_at_end(89, 10, 100));
    }

    #[test]
    fn keyboard_targets_preserve_navigation_math() {
        assert_eq!(keyboard_target_line(ARROW_UP, 1, 20), Some(0));
        assert_eq!(keyboard_target_line(PAGE_UP, 10, 20), Some(0));
        assert_eq!(keyboard_target_line(ARROW_DOWN, 10, 20), Some(11));
        assert_eq!(keyboard_target_line(PAGE_DOWN, 10, 20), Some(30));
        assert_eq!(keyboard_target_line("g", 10, 20), Some(0));
    }

    #[test]
    fn keyboard_targets_exclude_commands_and_unhandled_keys() {
        assert_eq!(keyboard_target_line("G", 10, 20), None);
        assert_eq!(keyboard_target_line("f", 10, 20), None);
        assert_eq!(keyboard_target_line("F", 10, 20), None);
        assert_eq!(keyboard_target_line("Enter", 10, 20), None);
    }

    #[test]
    fn wheel_target_uses_caller_base_and_clamps() {
        assert_eq!(wheel_target_line(50, -12, 100), 38);
        assert_eq!(wheel_target_line(5, -12, 100), 0);
        assert_eq!(wheel_target_line(95, 12, 100), 100);
        assert_eq!(wheel_target_line(25, 12, 100), 37);
    }

    #[test]
    fn focus_request_requires_new_nonzero_main_request() {
        assert!(should_handle_focus_request(
            2,
            1,
            SelectionSource::Main,
            SelectionSource::Main,
        ));
        assert!(!should_handle_focus_request(
            0,
            1,
            SelectionSource::Main,
            SelectionSource::Main,
        ));
        assert!(!should_handle_focus_request(
            2,
            2,
            SelectionSource::Main,
            SelectionSource::Main,
        ));
        assert!(!should_handle_focus_request(
            2,
            1,
            SelectionSource::Filter,
            SelectionSource::Main,
        ));
        assert!(!should_handle_focus_request(
            2,
            1,
            SelectionSource::Main,
            SelectionSource::Filter,
        ));
    }

    #[test]
    fn tail_update_disables_tail_and_follow_on_backward_navigation() {
        assert_eq!(
            tail_update_for_navigation(50, 49, 10, 100, TailEndComparison::Inclusive),
            TailNavigationUpdate::DisableTailAndFollow
        );
        assert_eq!(
            tail_update_for_navigation(50, 49, 10, 100, TailEndComparison::Strict),
            TailNavigationUpdate::DisableTailAndFollow
        );
    }

    #[test]
    fn tail_update_preserves_inclusive_and_strict_end_semantics() {
        assert_eq!(
            tail_update_for_navigation(80, 90, 10, 100, TailEndComparison::Inclusive),
            TailNavigationUpdate::EnableTail
        );
        assert_eq!(
            tail_update_for_navigation(80, 90, 10, 100, TailEndComparison::Strict),
            TailNavigationUpdate::NoChange
        );
        assert_eq!(
            tail_update_for_navigation(80, 91, 10, 100, TailEndComparison::Strict),
            TailNavigationUpdate::EnableTail
        );
        assert_eq!(
            tail_update_for_navigation(80, 89, 10, 100, TailEndComparison::Inclusive),
            TailNavigationUpdate::NoChange
        );
    }
}
