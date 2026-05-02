use crate::components::context::LogViewContext;
use crate::components::progress_bar::ProgressBar;
use leptos::prelude::*;
use leptos::{component, view, IntoView};

#[component]
pub fn PaneIndexProgress(
    context: LogViewContext,
    #[prop(into)] hidden: Signal<bool>,
    #[prop(optional)] variant_class: &'static str,
) -> impl IntoView {
    let LogViewContext {
        log_page,
        indexing_progress,
        set_indexing_progress,
        ..
    } = context;

    Effect::new(move || {
        if let Some(Ok(page_result)) = log_page.get() {
            set_indexing_progress.set(page_result.indexing_progress);
        }
    });

    view! {
        <ProgressBar
            progress=indexing_progress
            hidden=hidden
            variant_class=variant_class
        />
    }
}
