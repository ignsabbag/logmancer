use crate::components::context::LogViewContext;
use leptos::context::use_context;
use leptos::prelude::{ClassAttribute, Effect, ElementChild, Get, NodeRef, NodeRefAttribute, Set, Suspend, Transition};
use leptos::{component, html, view, IntoView};

#[component]
pub fn ContentLines() -> impl IntoView {
    let LogViewContext {
        page_size,
        set_page_size,
        log_page,
        ..
    } = use_context().expect("");

    let div_ref: NodeRef<html::Div> = NodeRef::new();

    Effect::new(move || {
        if let Some(div) = div_ref.get() {
            let mut lines = (div.client_height() as f32 / 20.0) as usize;
            lines = lines.saturating_sub(1);
            if lines != page_size.get() {
                set_page_size.set(lines);
            }
        }
    });
    
    view! {
        <div node_ref=div_ref class="content-lines">
            <Transition fallback=move || view! { <p>"Loading..."</p> }>
                <ul>
                    { move || Suspend::new(async move {
                        log_page.await.map(|page_result| view! {
                            { page_result.lines.into_iter().enumerate().map(|(i, line)| view! {
                                <li><b>{page_result.start_line + i + 1}</b> | {line}</li>
                            }).collect::<Vec<_>>() }
                        })
                    })}
                </ul>
            </Transition>
        </div>
    }
}