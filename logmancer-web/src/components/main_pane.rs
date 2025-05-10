use crate::components::content_lines::ContentLines;
use crate::components::content_scroll::ContentScroll;
use leptos::prelude::{ClassAttribute, ElementChild};
use leptos::{component, view, IntoView};

#[component]
pub fn MainPane() -> impl IntoView {
    
    view! {
        <div class="main-pane">
            <div class="content">
                <ContentLines />
                <ContentScroll />
            </div>
        </div>
    }
}