use crate::components::context::{LogFileContext, LogViewContext};
use crate::components::diagnostics::{scroll_trace, scroll_trace_enabled};
use crate::components::layout::{
    LOG_LINE_HEIGHT_PX, VIRTUAL_SCROLL_BASE_LINES, VIRTUAL_SCROLL_MAX_SPACER_HEIGHT_PX,
};
use leptos::context::use_context;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::{component, html, view, IntoView};
use logmancer_core::PageResult;
use std::time::Duration;

#[component]
pub fn ContentScroll(context: LogViewContext) -> impl IntoView {
    let LogFileContext { set_tail, .. } = use_context().expect("");

    let LogViewContext {
        set_start_line,
        page_size,
        log_page,
        ..
    } = context;

    let scroll_ref: NodeRef<html::Div> = NodeRef::new();
    let spacer_ref: NodeRef<html::Div> = NodeRef::new();

    let (programmatic_scroll, set_programmatic_scroll) = signal(false);
    let (page_result, set_page_result) = signal(None::<PageResult>);
    let (scroll_debounce, set_scroll_debounce) = signal::<Option<TimeoutHandle>>(None);
    let (index_debounce, set_index_debounce) = signal::<Option<TimeoutHandle>>(None);
    let (scroll_event_id, set_scroll_event_id) = signal(0_u64);
    let (programmatic_event_id, set_programmatic_event_id) = signal(0_u64);
    let scroll_trace = scroll_trace_enabled();

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
            let event_id = scroll_event_id.get_untracked().saturating_add(1);
            set_scroll_event_id.set(event_id);
            let is_programmatic = programmatic_scroll.get();
            scroll_trace!(
                scroll_trace,
                "scroll-trace scrollbar event_id={} programmatic={} scroll_top={} scroll_height={} client_height={} debounce_active={}",
                event_id,
                is_programmatic,
                scroll.scroll_top(),
                scroll.scroll_height(),
                scroll.client_height(),
                scroll_debounce.get_untracked().is_some()
            );
            if is_programmatic {
                return;
            }

            if let Some(page_result) = page_result.get() {
                if scroll_debounce.get().is_none() {
                    scroll_trace!(
                        scroll_trace,
                        "scroll-trace scrollbar schedule event_id={} captured_start_line={} total_lines={} page_size={}",
                        event_id,
                        page_result.start_line,
                        page_result.total_lines,
                        page_size.get_untracked()
                    );
                    let timeout_handle = set_timeout_with_handle(
                        move || {
                            let ratio = page_result.total_lines as f64 * scroll.scroll_top() as f64
                                / scroll.scroll_height() as f64;
                            let approx_line = ratio.floor() as usize;
                            scroll_trace!(
                                scroll_trace,
                                "scroll-trace scrollbar fire event_id={} captured_start_line={} approx_line={} scroll_top={} scroll_height={} client_height={}",
                                event_id,
                                page_result.start_line,
                                approx_line,
                                scroll.scroll_top(),
                                scroll.scroll_height(),
                                scroll.client_height()
                            );
                            if page_result.start_line != approx_line {
                                scroll_trace!(
                                    scroll_trace,
                                    "scroll-trace scrollbar apply event_id={} start_line_before={} start_line_after={}",
                                    event_id,
                                    page_result.start_line,
                                    approx_line
                                );
                                update_tail(approx_line, page_result);
                                set_start_line.set(approx_line);
                            }
                            set_scroll_debounce.set(None);
                        },
                        Duration::from_millis(300),
                    )
                    .ok();
                    set_scroll_debounce.set(timeout_handle);
                }
            }
        }
    };

    let spacer_height = Memo::new(move |_| {
        if let Some(page_result) = page_result.get() {
            calculate_spacer_height(page_result.total_lines)
        } else {
            0
        }
    });

    let scroll_pos = Memo::new(move |_| {
        if let Some(page_result) = page_result.get() {
            scroll_trace!(
                scroll_trace,
                "scroll-trace position height={} start_line={} total_lines={}",
                spacer_height.get(),
                page_result.start_line,
                page_result.total_lines
            );
            let ratio = page_result.start_line as f64 / page_result.total_lines as f64;
            (ratio * spacer_height.get() as f64).ceil() as i32
        } else {
            0
        }
    });

    Effect::new(move || {
        if let Some(scroll) = scroll_ref.get() {
            let event_id = programmatic_event_id.get_untracked().saturating_add(1);
            set_programmatic_event_id.set(event_id);
            scroll_trace!(
                scroll_trace,
                "scroll-trace programmatic apply event_id={} scroll_top_before={} scroll_top_after={} scroll_height={} client_height={}",
                event_id,
                scroll.scroll_top(),
                scroll_pos.get(),
                scroll.scroll_height(),
                scroll.client_height()
            );
            set_programmatic_scroll.set(true);
            scroll.set_scroll_top(scroll_pos.get());

            if let Some(spacer) = spacer_ref.get() {
                (*spacer)
                    .style()
                    .set_property("height", format!("{}px", spacer_height.get()).as_str())
                    .unwrap();
            }

            set_timeout(
                move || {
                    scroll_trace!(
                        scroll_trace,
                        "scroll-trace programmatic release event_id={}",
                        event_id
                    );
                    set_programmatic_scroll.set(false)
                },
                Duration::from_millis(50),
            );
        }
    });

    Effect::new(move || {
        if let Some(page_result) = page_result.get() {
            if page_result.indexing_progress < 1.0 && index_debounce.get().is_none() {
                let handle = set_timeout_with_handle(
                    move || {
                        set_start_line.notify();
                        set_index_debounce.set(None);
                    },
                    Duration::from_secs(1),
                )
                .ok();
                set_index_debounce.set(handle);
            }
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
/// - **Linear scaling** using the shared log line height for datasets ≤ 10,000 lines
/// - **Logarithmic scaling** for larger datasets to avoid overflow
///
/// # Behavior
/// - For 0-10k lines: Direct line-to-pixel mapping
/// - For 10k-500k lines: Gradual height increase (logarithmic phase)
/// - Above 1M lines: Hard cap at 1M pixels (browser safety)
fn calculate_spacer_height(lines: usize) -> usize {
    let lines_f64 = lines as f64;
    let height = if lines_f64 <= VIRTUAL_SCROLL_BASE_LINES {
        // Linear growth for lower values
        lines_f64 * LOG_LINE_HEIGHT_PX
    } else {
        // Logarithmic growth for large values
        VIRTUAL_SCROLL_BASE_LINES * LOG_LINE_HEIGHT_PX
            + (lines_f64 / VIRTUAL_SCROLL_BASE_LINES).ln() * VIRTUAL_SCROLL_MAX_SPACER_HEIGHT_PX
                / 2.0
    };
    height.min(VIRTUAL_SCROLL_MAX_SPACER_HEIGHT_PX) as usize
}
