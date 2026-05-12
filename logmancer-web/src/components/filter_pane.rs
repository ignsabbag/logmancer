use crate::components::async_functions::{apply_filter as apply_filter_fetch, fetch_filter_page};
use crate::components::content_lines::ContentLines;
use crate::components::content_scroll::ContentScroll;
use crate::components::context::{
    ActivePaneContext, LogFileContext, LogViewContext, SelectionContext, SelectionSource,
};
use crate::components::layout::LOG_LINE_HEIGHT_PX;
use crate::components::pane_index_progress::PaneIndexProgress;
use leptos::context::use_context;
use leptos::ev::KeyboardEvent;
use leptos::html::Div;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::{component, view, IntoView};

#[component]
pub fn FilterPane() -> impl IntoView {
    let LogFileContext { file_id, .. } = use_context().expect("LogFileContext not found");

    let div_ref = NodeRef::<Div>::new();
    let (content_width, set_content_width) = signal(2048_f64);
    let (content_height, set_content_height) = signal(1080_f64);

    let (filter_text, set_filter_text) = signal(String::new());
    let (filter_applied, set_filter_applied) = signal(false);
    let (indexing_progress, set_indexing_progress) = signal(0_f64);
    let SelectionContext {
        selected_original_line,
        set_selected_original_line,
        set_selected_line_source,
        ..
    } = use_context().expect("SelectionContext not found");
    let ActivePaneContext {
        active_pane,
        set_active_pane,
    } = use_context().expect("ActivePaneContext not found");

    let (start_line, set_start_line) = signal(0_usize);
    let (page_size, set_page_size) = signal(50_usize);

    let filter_page = LocalResource::new(move || {
        let file_id = file_id.get();
        let start = start_line.get();
        let size = page_size.get();
        let applied = filter_applied.get();

        async move {
            if applied {
                fetch_filter_page(file_id, start, size).await
            } else {
                Err(ServerFnError::Request(String::new()))
            }
        }
    });

    let log_view_context = LogViewContext {
        set_start_line,
        page_size,
        set_page_size,
        log_page: filter_page,
        indexing_progress,
        set_indexing_progress,
        selected_line: selected_original_line,
        set_selected_line: set_selected_original_line,
        selection_source: SelectionSource::Filter,
        set_selected_line_source,
        set_active_pane,
    };

    let on_input = move |ev: leptos::ev::Event| {
        let value = event_target_value(&ev);
        set_filter_text.set(value);
    };

    let on_key_down = move |ev: KeyboardEvent| {
        set_active_pane.set(SelectionSource::Filter);
        if ev.key() == "Enter" {
            let text = filter_text.get();
            if !text.is_empty() {
                let file_id = file_id.get();
                let text_clone = text.clone();

                spawn_local(async move {
                    apply_filter_fetch(file_id, text_clone).await.ok();
                    set_filter_applied.set(true);
                    set_indexing_progress.set(0.0);
                    // Reset scroll position when filter changes
                    set_start_line.set(0);
                });
            } else {
                set_filter_applied.set(false);
                set_indexing_progress.set(0.0);
            }
        }
    };

    Effect::new(move || {
        use leptos_use::use_resize_observer;
        use_resize_observer(div_ref, move |entries, _observer| {
            let rect = entries[0].content_rect();
            if content_width.get() != rect.width() {
                set_content_width.set(rect.width());
            }
            if content_height.get() != rect.height() {
                set_content_height.set(rect.height());

                let lines = (rect.height() / LOG_LINE_HEIGHT_PX) as usize;
                if lines != page_size.get() && lines > 0 {
                    set_page_size.set(lines);
                }
            }
        });
    });

    // Re-fetch when filter changes
    Effect::new(move || {
        filter_applied.track();
        set_start_line.notify();
    });

    let filter_progress_hidden =
        Signal::derive(move || !filter_applied.get() || indexing_progress.get() >= 1.0);

    view! {
        <div
            class="filter-pane"
            class:active-pane=move || active_pane.get() == SelectionSource::Filter
        >
            <div class="filter-input-container">
                <input
                    type="text"
                    class="filter-input"
                    placeholder="Filter (press Enter)"
                    value=filter_text
                    on:input=on_input
                    on:keydown=on_key_down
                    on:focus=move |_| set_active_pane.set(SelectionSource::Filter)
                />
            </div>
            <PaneIndexProgress
                context=log_view_context.clone()
                hidden=filter_progress_hidden
                variant_class="progress-bar--local"
            />
            <div node_ref=div_ref class="content">
                <ContentLines context=log_view_context.clone() />
                <ContentScroll context=log_view_context.clone() />
            </div>
        </div>
    }
}
