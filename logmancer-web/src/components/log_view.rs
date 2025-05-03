use crate::api::commons::ReadPageRequest;
use leptos::prelude::PropAttribute;
use leptos::prelude::{signal, window, Get, LocalResource, Suspend, Transition};
use leptos::prelude::{ClassAttribute, ServerFnError};
use leptos::prelude::ElementChild;
use leptos::prelude::{OnTargetAttribute, Set, StyleAttribute};
use leptos::{component, view, IntoView};
use leptos_router::hooks::use_params_map;
use logmancer_core::PageResult;

#[component]
pub fn LogView() -> impl IntoView {
    let params = use_params_map();
    let page_size = move || 50_usize;
    let file_id = move || params.get().get("id").unwrap_or_default();
    let (start_line, set_start_line) = signal(0usize);

    let log_page = LocalResource::new(
        move || fetch_page(file_id(), start_line.get(), page_size()));

    // provide_context(LogViewContext {
    //     file_id,
    //     start_line,
    //     set_start_line,
    //     page_size,
    //     log_page
    // });
    view! {
        <input type="text" prop:value=start_line
        on:input:target=move |ev| {
            if let Ok(new_line) = ev.target().value().parse::<usize>() {
                if new_line > 0 {
                    set_start_line.set(new_line);
                }
            }
        } />
        <Transition fallback=move || view! { <p>"Loading..."</p> }>
            <div class="overflow-auto h-[400px] w-full border font-mono whitespace-pre">
                { move || Suspend::new(async move {
                    log_page.await.map(|page_result| view! {
                        <h2>Total lines: {page_result.total_lines} - {page_result.indexing_progress * 100_f64}%</h2>
                        <div style="text-align: left;">
                        { page_result.lines.into_iter().map(|line| view! {
                                <span>{line}</span>
                            }).collect::<Vec<_>>() }
                        </div>
                    })
                })}
            </div>
        </Transition>
    }
}

async fn fetch_page(file_id: String, start_line: usize, max_lines: usize) -> Result<PageResult, ServerFnError> {
    let base = window().location().origin().unwrap();
    let url = format!("{}/api/read-page", base);
    let result = reqwest::Client::new()
        .get(url)
        .query(&ReadPageRequest {
            file_id,
            start_line,
            max_lines
        })
        .send()
        .await.map_err(|e| ServerFnError::WrappedServerError(e))?
        .json::<PageResult>()
        .await.map_err(|e| ServerFnError::WrappedServerError(e))?;
    Ok(result)
}