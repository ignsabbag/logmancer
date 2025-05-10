use crate::components::context::LogViewContext;
use leptos::context::use_context;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::{component, html, view, IntoView};

const MAX_SPACER_HEIGHT: f64 = 1_000_000.0;
const BASE_LINES: f64 = 10_000.0;
const LINE_HEIGHT: f64 = 20.0;

#[component]
pub fn ContentScroll() -> impl IntoView {
    let LogViewContext {
        start_line,
        set_start_line,
        total_lines,
        set_total_lines,
        log_page,
        ..
    } = use_context().expect("");

    let scroll_ref: NodeRef<html::Div> = NodeRef::new();
    let spacer_ref: NodeRef<html::Div> = NodeRef::new();

    let (is_programmatic_scroll, set_is_programmatic_scroll) = signal(false);
    let (timeout, set_timeout) = signal::<Option<TimeoutHandle>>(None);
    let (last_start_line, set_last_start_line) = signal(1);

    let on_scroll = move |_| {
        if is_programmatic_scroll.get() { return; }
        if let Some(scroll) = scroll_ref.get() {
            log!("Scroll detected");
            if let Some(timeout_handle) = timeout.get() {
                log!("Timeout found. Cleaning..");
                timeout_handle.clear();
            }
            if let Ok(handle) = set_timeout_with_handle(
                move || {
                    let ratio = total_lines.get() as f64 * scroll.scroll_top() as f64 / scroll.scroll_height() as f64;
                    let approx_line = ratio.floor() as usize;
                    log!("Scrolling to line {}", approx_line);
                    if start_line.get() != approx_line {
                        log!("Updating start_line. Old: {}. New: {}", start_line.get(), approx_line);
                        set_start_line.set(approx_line);
                    }
                },
                std::time::Duration::from_millis(300)
            ) {
                log!("Timeout set");
                set_timeout.set(Some(handle));
            }
        }
    };

    Effect::new(move |_| {
        log!("Effect enter");
        if let Some(Ok(page_result)) = log_page.get().as_deref().map(|res| res.as_ref()) {
            if page_result.start_line != last_start_line.get() || page_result.total_lines != total_lines.get() {
                set_is_programmatic_scroll.set(true);
                set_last_start_line.set(page_result.start_line);
                log!("New page_result. Start line: {}. Total lines: {}", page_result.start_line, page_result.total_lines);
                let height = calculate_spacer_height(page_result.total_lines);
                let ratio = page_result.start_line as f64 / page_result.total_lines as f64;
                let scroll_pos = ratio * height as f64;
                if let Some(spacer) = spacer_ref.get() {
                    (*spacer).style().set_property("height", format!("{}px", height).as_str()).unwrap();
                    log!("Actualizado el height de spacer a {}", height);
                }
                if let Some(scroll) = scroll_ref.get() {
                    (*scroll).set_scroll_top(scroll_pos.ceil() as i32);
                    log!("Actualizado el scroll top a {}", scroll_pos);
                }
                if start_line.get() != page_result.start_line {
                    log!("Updating start_line. Old: {}. New: {}", start_line.get(), page_result.start_line);
                    set_start_line.set(page_result.start_line);
                }
                if total_lines.get() != page_result.total_lines {
                    log!("Updating total_lines. Old: {}. New: {}", total_lines.get(), page_result.total_lines);
                    set_total_lines.set(page_result.total_lines);
                }
                set_is_programmatic_scroll.set(false);
            }
        }
        log!("Effect end");
    });
    
    view! {
        <div node_ref=scroll_ref class="scrollbar" on:scroll=on_scroll>
            <div class="spacer" node_ref=spacer_ref></div>
        </div>
    }
}

/// Calculates the proportional height for the scroll spacer element to emulate virtual scrolling.
/// 
/// # Algorithm
/// Uses a hybrid approach to balance precision and performance:
/// - **Linear scaling** (`20px/line`) for datasets ≤ 10,000 lines
/// - **Logarithmic scaling** for larger datasets to avoid overflow
///
/// # Behavior
/// - For 0-10k lines: Direct 1:20 line-to-pixel mapping
/// - For 10k-500k lines: Gradual height increase (logarithmic phase)
/// - Above 1M lines: Hard cap at 1M pixels (browser safety)
fn calculate_spacer_height(lines: usize) -> usize {
    let lines_f64 = lines as f64;
    let height = if lines_f64 <= BASE_LINES {
        // Linear growth for lower values
        lines_f64 * LINE_HEIGHT
    } else {
        // Logarithmic growth for large values
        BASE_LINES * LINE_HEIGHT + (lines_f64 / BASE_LINES).ln() * MAX_SPACER_HEIGHT / 2.0
    };
    height.min(MAX_SPACER_HEIGHT) as usize
}