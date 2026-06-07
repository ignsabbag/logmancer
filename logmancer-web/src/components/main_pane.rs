use crate::components::async_functions::{
    apply_search, clear_search, fetch_page, search_next, search_previous,
};
use crate::components::auto_scroll_status::AutoScrollStatus;
use crate::components::content_lines::ContentLines;
use crate::components::content_scroll::ContentScroll;
use crate::components::context::{
    ActivePaneContext, LogContentFocusContext, LogFileContext, LogViewContext,
    SearchCommandContext, SearchUiContext, SelectionContext, SelectionSource,
};
use crate::components::layout::LOG_LINE_HEIGHT_PX;
use crate::components::pane_index_progress::PaneIndexProgress;
use crate::components::search_status::format_page_search_status;
use leptos::context::use_context;
use leptos::html::Div;
use leptos::leptos_dom::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::{component, view, IntoView};
use leptos_use::use_resize_observer;
use logmancer_core::PageResult;

fn reveal_start_line_for_selected_line(selected_original_line: usize, page_size: usize) -> usize {
    if selected_original_line == 0 {
        return 0;
    }

    let selected_zero_based = selected_original_line.saturating_sub(1);
    let center_offset = page_size / 2;
    selected_zero_based.saturating_sub(center_offset)
}

fn apply_search_page_result(
    page: PageResult,
    set_tail: WriteSignal<bool>,
    set_follow: WriteSignal<bool>,
    set_start_line: WriteSignal<usize>,
    set_selected_original_line: WriteSignal<Option<usize>>,
    set_selected_line_source: WriteSignal<SelectionSource>,
) {
    set_tail.set(false);
    set_follow.set(false);
    set_start_line.set(page.start_line);
    set_start_line.notify();

    let selected_match = page
        .search
        .as_ref()
        .and_then(|search| search.current.as_ref().or(search.first.as_ref()))
        .cloned();

    set_selected_line_source.set(SelectionSource::Main);
    set_selected_original_line.set(selected_match.map(|search_match| search_match.line_index + 1));
}

#[component]
pub fn MainPane() -> impl IntoView {
    let LogFileContext {
        file_id,
        tail,
        set_tail,
        follow,
        set_follow,
        ..
    } = use_context().expect("");

    let SelectionContext {
        selected_original_line,
        set_selected_original_line,
        selected_line_source,
        set_selected_line_source,
    } = use_context().expect("SelectionContext not found");

    let ActivePaneContext {
        active_pane,
        set_active_pane,
    } = use_context().expect("ActivePaneContext not found");
    let SearchUiContext {
        query: search_query,
        set_status: set_search_status,
        ..
    } = use_context().expect("SearchUiContext not found");
    let SearchCommandContext {
        submit_request: search_submit_request,
        clear_request: search_clear_request,
        next_request: search_next_request,
        previous_request: search_previous_request,
        navigation_in_flight: search_navigation_in_flight,
        set_navigation_in_flight: set_search_navigation_in_flight,
        ..
    } = use_context().expect("SearchCommandContext not found");
    let LogContentFocusContext {
        request_focus: request_log_content_focus,
        ..
    } = use_context().expect("LogContentFocusContext not found");

    let div_ref = NodeRef::<Div>::new();
    let (content_width, set_content_width) = signal(2048_f64);
    let (content_height, set_content_height) = signal(1080_f64);

    let (start_line, set_start_line) = signal(0_usize);
    let (page_size, set_page_size) = signal(50_usize);
    let (indexing_progress, set_indexing_progress) = signal(0_f64);
    let (handled_next_request, set_handled_next_request) = signal(0_u64);
    let (handled_previous_request, set_handled_previous_request) = signal(0_u64);
    let log_page = LocalResource::new(move || {
        fetch_page(
            file_id.get(),
            start_line.get(),
            page_size.get(),
            tail.get(),
            follow.get(),
        )
    });

    let log_view_context = LogViewContext {
        set_start_line,
        page_size,
        set_page_size,
        log_page,
        indexing_progress,
        set_indexing_progress,
        selected_line: selected_original_line,
        set_selected_line: set_selected_original_line,
        selection_source: SelectionSource::Main,
        set_selected_line_source,
        set_active_pane,
    };

    let return_focus_to_main = move || {
        set_active_pane.set(SelectionSource::Main);
        request_log_content_focus.update(|request| *request = request.saturating_add(1));
    };

    let clear_current_search = move || {
        let file_id = file_id.get_untracked();

        spawn_local(async move {
            clear_search(file_id).await.ok();
            set_selected_original_line.set(None);
            set_start_line.notify();
            set_search_status.set(String::new());
            return_focus_to_main();
        });
    };

    let submit_search = move || {
        let query = search_query.get_untracked().trim().to_string();
        let file_id = file_id.get_untracked();
        let max_lines = page_size.get_untracked();

        if query.is_empty() {
            clear_current_search();
            return;
        }

        set_search_status.set("Searching...".to_string());
        spawn_local(async move {
            match apply_search(file_id, query, max_lines).await {
                Ok(page) => {
                    set_search_status.set(format_page_search_status(&page));
                    apply_search_page_result(
                        page,
                        set_tail,
                        set_follow,
                        set_start_line,
                        set_selected_original_line,
                        set_selected_line_source,
                    );
                    return_focus_to_main();
                }
                Err(_) => {
                    set_search_status.set("Search failed".to_string());
                    return_focus_to_main();
                }
            }
        });
    };

    let navigate_search = move |navigate_previous: bool| {
        let file_id = file_id.get_untracked();
        let max_lines = page_size.get_untracked();

        set_search_navigation_in_flight.set(true);

        set_search_status.set(if navigate_previous {
            "Going to previous match...".to_string()
        } else {
            "Going to next match...".to_string()
        });

        spawn_local(async move {
            let result = if navigate_previous {
                search_previous(file_id, max_lines).await
            } else {
                search_next(file_id, max_lines).await
            };

            match result {
                Ok(page) => {
                    set_search_status.set(format_page_search_status(&page));
                    apply_search_page_result(
                        page,
                        set_tail,
                        set_follow,
                        set_start_line,
                        set_selected_original_line,
                        set_selected_line_source,
                    );
                }
                Err(_) => {
                    set_search_status.set(if navigate_previous {
                        "Previous match unavailable".to_string()
                    } else {
                        "Next match unavailable".to_string()
                    });
                }
            }

            set_search_navigation_in_flight.set(false);
            return_focus_to_main();
        });
    };

    Effect::new(move || {
        if selected_line_source.get() != SelectionSource::Filter {
            return;
        }

        if let Some(line_number) = selected_original_line.get() {
            let reveal_start_line =
                reveal_start_line_for_selected_line(line_number, page_size.get());
            set_tail.set(false);
            set_follow.set(false);
            set_start_line.set(reveal_start_line);
        }
    });

    Effect::new(move || {
        let request = search_submit_request.get();
        if request == 0 {
            return;
        }

        submit_search();
    });

    Effect::new(move || {
        let request = search_clear_request.get();
        if request == 0 {
            return;
        }

        clear_current_search();
    });

    Effect::new(move || {
        let request = search_next_request.get();
        let handled_request = handled_next_request.get();

        if request == 0 || handled_request >= request {
            return;
        }

        if search_navigation_in_flight.get() {
            return;
        }

        set_handled_next_request.update(|handled| *handled = handled.saturating_add(1));
        navigate_search(false);
    });

    Effect::new(move || {
        let request = search_previous_request.get();
        let handled_request = handled_previous_request.get();

        if request == 0 || handled_request >= request {
            return;
        }

        if search_navigation_in_flight.get() {
            return;
        }

        set_handled_previous_request.update(|handled| *handled = handled.saturating_add(1));
        navigate_search(true);
    });

    Effect::new(move || {
        use_resize_observer(div_ref, move |entries, _observer| {
            let rect = entries[0].content_rect();
            if content_width.get() != rect.width() {
                log!("Updating content width to {}", rect.width());
                set_content_width.set(rect.width());
            }
            if content_height.get() != rect.height() {
                log!("Updating content height to {}", rect.height());
                set_content_height.set(rect.height());

                let lines = (rect.height() / LOG_LINE_HEIGHT_PX) as usize;
                if lines != page_size.get() {
                    log!("Updating page_size to {}", lines);
                    set_page_size.set(lines);
                }
            }
        });
    });

    view! {
        <div
            class="main-pane"
            class:active-pane=move || active_pane.get() == SelectionSource::Main
        >
            <PaneIndexProgress
                context=log_view_context.clone()
                hidden=Signal::derive(move || indexing_progress.get() >= 1.0)
                variant_class="progress-bar--global"
            />
            <div node_ref=div_ref class="content">
                <ContentLines context=log_view_context.clone() />
                <ContentScroll context=log_view_context.clone() />
            </div>
            <AutoScrollStatus />
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::reveal_start_line_for_selected_line;

    #[test]
    fn reveal_line_at_top_clamps_to_zero() {
        assert_eq!(reveal_start_line_for_selected_line(1, 50), 0);
    }

    #[test]
    fn reveal_center_for_middle_line() {
        assert_eq!(reveal_start_line_for_selected_line(101, 50), 75);
    }

    #[test]
    fn reveal_handles_even_page_size_off_by_one() {
        assert_eq!(reveal_start_line_for_selected_line(26, 50), 0);
        assert_eq!(reveal_start_line_for_selected_line(27, 50), 1);
    }
}
