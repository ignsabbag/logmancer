use leptos::prelude::ElementChild;
use leptos::{component, view, IntoView};
use leptos::prelude::ClassAttribute;
use crate::components::lines::Lines;
use crate::components::position_slider::PositionSlider;

#[component]
pub fn MainPane() -> impl IntoView {
    view! {
        <div class="overflow-auto h-[400px] w-full border font-mono whitespace-pre">
            <Lines />
            <PositionSlider />
        </div>
    }
}