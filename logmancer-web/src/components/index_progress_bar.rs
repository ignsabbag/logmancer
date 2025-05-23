use crate::components::context::LogViewContext;
use leptos::context::use_context;
use leptos::prelude::*;
use leptos::*;
use std::time::Duration;

#[component]
pub fn IndexProgressBar() -> impl IntoView {
    let LogViewContext {
        set_page_size,
        log_page,
        ..
    } = use_context().expect("");
    
    view! {
        <Transition>
            { move || Suspend::new(async move {
                log_page.await.map(|page_result| {
                    if page_result.indexing_progress < 1.0 {
                        set_timeout(move || set_page_size.notify(), Duration::from_secs(1))
                    }
                    view! {
                        <div
                            id="progress-bar"
                            class:hidden=move || { page_result.indexing_progress >= 1.0 }
                            style:width=move || { format!("{}%", page_result.indexing_progress * 100.0) }
                        ></div>
                    }
                })
            })}
        </Transition>
    }
}
