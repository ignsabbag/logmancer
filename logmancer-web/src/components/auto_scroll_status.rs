use crate::components::context::LogFileContext;
use leptos::context::use_context;
use leptos::prelude::*;
use leptos::{component, view, IntoView};

#[component]
pub fn AutoScrollStatus() -> impl IntoView {
    let LogFileContext { follow, .. } = use_context().expect("LogFileContext not found");

    view! {
        <div
            class="auto-scroll-status"
            class:auto-scroll-status--on=move || follow.get()
        >
            <span class="auto-scroll-status__dot"></span>
            <span>{move || if follow.get() { "AUTO ON" } else { "AUTO OFF" }}</span>
        </div>
    }
}
