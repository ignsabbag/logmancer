use std::time::Duration;
use crate::components::context::LogViewContext;
use leptos::context::use_context;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::{component, html, view, IntoView};
use logmancer_core::PageResult;

const MAX_SPACER_HEIGHT: f64 = 10_000_000.0;
const BASE_LINES: f64 = 10_000.0;
const LINE_HEIGHT: f64 = 20.0;

#[component]
pub fn ContentScroll() -> impl IntoView {
    let LogViewContext {
        set_start_line,
        page_size,
        set_tail,
        log_page,
        ..
    } = use_context().expect("");

    let scroll_ref: NodeRef<html::Div> = NodeRef::new();
    let spacer_ref: NodeRef<html::Div> = NodeRef::new();

    let (programmatic_scroll, set_programmatic_scroll) = signal(false);
    let (page_result, set_page_result) = signal(None::<PageResult>);
    let (debounce, set_debounce) = signal::<Option<TimeoutHandle>>(None);

    let update_tail = move |new_line: usize, page_result: PageResult| {
        set_tail.update_untracked(move |current| {
            if new_line < page_result.start_line {
                log!("Updating tail to false");
                *current = false
            } else if new_line + page_size.get() > page_result.total_lines {
                log!("Updating tail to true");
                *current = true
            }
        });
    };

    let on_scroll = move |_| {
        if let Some(scroll) = scroll_ref.get() {
            log!("Programmatic Scroll: {}", programmatic_scroll.get());
            if programmatic_scroll.get() { return; }

            if let Some(page_result) = page_result.get() {
                log!("Scroll detected: {}", scroll.scroll_top());
                if None == debounce.get() {
                    let timeout_handle = set_timeout_with_handle(
                        move || {
                            let ratio = page_result.total_lines as f64 * scroll.scroll_top() as f64 / scroll.scroll_height() as f64;
                            let approx_line = ratio.floor() as usize;
                            log!("Scrolling to line {}", approx_line);
                            if page_result.start_line != approx_line {
                                log!("Updating start_line by scrollbar. Old: {}. New: {}", page_result.start_line, approx_line);
                                update_tail(approx_line, page_result);
                                set_start_line.set(approx_line);
                            }
                            set_debounce.set(None);
                        },
                        Duration::from_millis(300)
                    ).ok();
                    set_debounce.set(timeout_handle);
                }
            }
        }
    };

    let spacer_height = Memo::new(move |_| {
        if let Some(page_result) = page_result.get() {
            calculate_spacer_height(page_result.total_lines)
        } else { 0 }
    });

    let scroll_pos = Memo::new(move |_| {
        if let Some(page_result) = page_result.get() {
            log!("Calculating scroll position. Height: {}. StartLine: {}. TotalLines: {}",
                    spacer_height.get(), page_result.start_line, page_result.total_lines);
            let ratio = page_result.start_line as f64 / page_result.total_lines as f64;
            (ratio * spacer_height.get() as f64).ceil() as i32
        } else { 0 }
    });

    Effect::new(move || {
        if let Some(scroll) = scroll_ref.get() {
            log!("Updating scroll_top to {}", scroll_pos.get());
            set_programmatic_scroll.set(true);
            scroll.set_scroll_top(scroll_pos.get());

            if let Some(spacer) = spacer_ref.get() {
                (*spacer).style().set_property("height", format!("{}px", spacer_height.get()).as_str()).unwrap();
            }

            set_timeout(move || set_programmatic_scroll.set(false), Duration::from_millis(50));
        }
    });

    view! {
        <div node_ref=scroll_ref class="scrollbar" on:scroll=on_scroll>
            <div class="spacer" node_ref=spacer_ref>
                <Transition>
                    { move || Suspend::new(async move {
                        log_page.await.map(|page_result| {
                            set_page_result.set(Some(page_result));
                        })
                    })}
                </Transition>
            </div>
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
    log!("Calculating spacer height");
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