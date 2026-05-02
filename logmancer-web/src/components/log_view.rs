use crate::components::context::LogFileContext;
use crate::components::filter_pane::FilterPane;
use crate::components::main_pane::MainPane;
use leptos::prelude::*;
use leptos::{component, view, IntoView};
use leptos_router::hooks::use_params_map;

#[component]
pub fn LogView() -> impl IntoView {
    let file_id = Memo::new(move |_| use_params_map().get().get("id").unwrap_or_default());
    let (follow, set_follow) = signal(false);
    let (tail, set_tail) = signal(false);

    provide_context(LogFileContext {
        file_id,
        tail,
        set_tail,
        follow,
        set_follow,
    });

    view! {
        <div class="log-view">
            <div class="main-pane-container">
                <MainPane />
            </div>
            <div class="divider"></div>
            <div class="filter-pane-container">
                <FilterPane />
            </div>
        </div>
    }
}
