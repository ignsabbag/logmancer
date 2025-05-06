use crate::components::context::LogViewContext;
use leptos::context::use_context;
use leptos::prelude::Transition;
use leptos::prelude::*;
use leptos::prelude::OnAttribute;
use leptos::{component, view, IntoView};

#[component]
pub fn PositionSlider() -> impl IntoView {
    let LogViewContext {
        set_start_line,
        page_size,
        log_page,
        ..
    } = use_context().expect("");

    view! {
        <div class="flex flex-col items-center mx-2">
            <Transition fallback=move || view! {
                <input type="range" min="0" max="0" value="0" 
                    class="h-[400px] rotate-[-90deg] w-[300px]" />
            }>
                { move || Suspend::new(async move {
                    log_page.await.map(|page_result| view! {
                        <input
                            type="range"
                            min=move || "0".to_string()
                            max=move || page_result.total_lines.saturating_sub(page_size.get())
                            value=move || page_result.start_line.to_string()
                            on:input=move |ev| {
                                let val = event_target_value(&ev);
                                if let Ok(pos) = val.parse::<usize>() {
                                    set_start_line.set(pos);
                                }
                            }
                            class="h-[400px] rotate-[-90deg] w-[300px]" />
                    })
                })}
            </Transition>
        </div>
    }
}