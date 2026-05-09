use crate::components::context::LogFileContext;
use leptos::context::use_context;
use leptos::prelude::*;
use leptos::{component, view, IntoView};

#[component]
pub fn AutoScrollStatus() -> impl IntoView {
    let LogFileContext { follow, .. } = use_context().expect("LogFileContext not found");

    view! {
        <div class="auto-scroll-status">
            {move || if follow.get() { "AUTO-SCROLL ON" } else { "AUTO-SCROLL OFF" }}
        </div>
    }
}
