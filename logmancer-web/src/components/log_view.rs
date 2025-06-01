use crate::components::context::LogFileContext;
use crate::components::index_progress_bar::IndexProgressBar;
use crate::components::main_pane::MainPane;
use leptos::prelude::*;
use leptos::{component, view, IntoView};
use leptos_router::hooks::use_params_map;
use crate::components::async_functions::fetch_info;

#[component]
pub fn LogView() -> impl IntoView {
    let file_id = Memo::new(move |_| {
        use_params_map().get().get("id").unwrap_or_default()
    });
    let (indexing_progress, set_indexing_progress) = signal(0_f64);
    let (follow, set_follow) = signal(false);
    let (tail, set_tail) = signal(false);

    let log_info = LocalResource::new(move || {
        indexing_progress.track();
        fetch_info(file_id.get())
    });

    provide_context(LogFileContext {
        file_id,
        indexing_progress,
        set_indexing_progress,
        tail,
        set_tail,
        follow,
        set_follow,
        log_info
    });

    view! {
        <IndexProgressBar />
        <MainPane />
    }
}