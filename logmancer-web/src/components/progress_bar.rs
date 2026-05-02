use leptos::prelude::*;
use leptos::{component, view, IntoView};

#[component]
pub fn ProgressBar(
    #[prop(into)] progress: Signal<f64>,
    #[prop(into)] hidden: Signal<bool>,
    #[prop(optional)] variant_class: &'static str,
) -> impl IntoView {
    view! {
        <div
            class=format!("progress-bar {}", variant_class)
            class:hidden=move || hidden.get()
            style:width=move || format!("{}%", (progress.get() * 100.0).clamp(0.0, 100.0))
        ></div>
    }
}
