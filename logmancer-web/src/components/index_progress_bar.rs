use leptos::*;
use leptos::context::use_context;
use leptos::prelude::{ClassAttribute, Get, Signal, StyleAttribute};
use crate::components::context::LogViewContext;

#[component]
pub fn IndexProgressBar() -> impl IntoView {
    let LogViewContext {
        index_progress,
        ..
    } = use_context().expect("");
    
    let hidden = Signal::derive(move || index_progress.get() >= 1.0);

    view! {
        <div
            class="progress-bar"
            class:hidden=hidden
            style:width=move || format!("{}%", index_progress.get() * 100.0)
        ></div>
    }
}
