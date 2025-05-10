use crate::components::lines::Lines;
use crate::components::position_slider::PositionSlider;
use leptos::prelude::{ElementChild, StyleAttribute};
use leptos::{component, view, IntoView};

#[component]
pub fn MainPane() -> impl IntoView {
    
    view! {
        <div style="height: 100vh; overflow: hidden; font-family: monospace;">
            <div style="display: flex; height: 100%; width: 100%;">
                <Lines />
                <PositionSlider />
            </div>
        </div>
    }
}