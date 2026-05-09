use crate::components::context::{
    ActivePaneContext, LogFileContext, SelectionContext, SelectionSource,
};
use crate::components::filter_pane::FilterPane;
use crate::components::main_pane::MainPane;
use leptos::html;
use leptos::prelude::*;
use leptos::{component, view, IntoView};
use leptos_router::hooks::use_params_map;

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
    let log_view_ref: NodeRef<html::Div> = NodeRef::new();

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
        >
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
                <MainPane />
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
                <FilterPane />
            </div>
        </div>
    }
}
