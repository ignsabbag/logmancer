use leptos::prelude::{ElementChild, StyleAttribute, Suspend, Transition};
use leptos::{component, view, IntoView};
use leptos::context::use_context;
use crate::components::context::LogViewContext;

#[component]
pub fn Lines() -> impl IntoView {
    let LogViewContext {
        log_page,
        ..
    } = use_context().expect("");
    view! {
        <Transition fallback=move || view! { <p>"Loading..."</p> }>
            <ul style="list-style: none; width=100%; max-width=100%; overflow-x: auto; white-space: nowrap; font-family: monospace; text-align: left;">
                { move || Suspend::new(async move {
                    log_page.await.map(|page_result| view! {
                        { page_result.lines.into_iter().enumerate().map(|(i, line)| view! {
                            <li><b>{page_result.start_line + i}</b>| {line}</li>
                        }).collect::<Vec<_>>() }
                    })
                })}
            </ul>
        </Transition>
    }
}