use crate::components::async_functions::fetch_page;
use crate::components::auto_scroll_status::AutoScrollStatus;
use crate::components::content_lines::ContentLines;
use crate::components::content_scroll::ContentScroll;
use crate::components::context::{
    ActivePaneContext, LogFileContext, LogViewContext, SelectionContext, SelectionSource,
};
use crate::components::layout::LOG_LINE_HEIGHT_PX;
use crate::components::pane_index_progress::PaneIndexProgress;
use leptos::context::use_context;
use leptos::html::Div;
use leptos::leptos_dom::log;
use leptos::prelude::*;
use leptos::{component, view, IntoView};
use leptos_use::use_resize_observer;

fn reveal_start_line_for_selected_line(selected_original_line: usize, page_size: usize) -> usize {
    if selected_original_line == 0 {
        return 0;
    }

    let selected_zero_based = selected_original_line.saturating_sub(1);
    let center_offset = page_size / 2;
    selected_zero_based.saturating_sub(center_offset)
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

    let div_ref = NodeRef::<Div>::new();
    let (content_width, set_content_width) = signal(2048_f64);
    let (content_height, set_content_height) = signal(1080_f64);

    let (start_line, set_start_line) = signal(0_usize);
    let (page_size, set_page_size) = signal(50_usize);
    let (indexing_progress, set_indexing_progress) = signal(0_f64);
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
