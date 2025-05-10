use leptos::prelude::{Effect, ElementChild, Get, NodeRef, NodeRefAttribute, Set, StyleAttribute, Suspend, Transition};
use leptos::{component, html, view, IntoView};
use leptos::context::use_context;
use crate::components::context::LogViewContext;

#[component]
pub fn Lines() -> impl IntoView {
    let LogViewContext {
        page_size,
        set_page_size,
        log_page,
        ..
    } = use_context().expect("");

    let div_ref: NodeRef<html::Div> = NodeRef::new();

    Effect::new(move || {
        if let Some(div) = div_ref.get() {
            let lines = (div.client_height() as f32 / 20.0) as usize - 1;
            if lines != page_size.get() {
                set_page_size.set(lines);
            }
        }
    });
    
    view! {
        <div node_ref=div_ref style="flex: 1; padding: 8px; overflow: hidden; line-height: 20px; white-space: pre; background: #1e1e1e; color: #dcdcdc;">
            <Transition fallback=move || view! { <p>"Loading..."</p> }>
                <ul style="list-style: none; font-family: monospace; text-align: left;">
                    { move || Suspend::new(async move {
                        log_page.await.map(|page_result| view! {
                            { page_result.lines.into_iter().enumerate().map(|(i, line)| view! {
                                <li><b>{page_result.start_line + i + 1}</b>| {line}</li>
                            }).collect::<Vec<_>>() }
                        })
                    })}
                </ul>
            </Transition>
        </div>
    }
}