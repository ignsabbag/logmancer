use crate::api::commons::ReadPageRequest;
use crate::components::context::LogViewContext;
use crate::components::main_pane::MainPane;
use leptos::prelude::provide_context;
use leptos::prelude::{signal, window, Get, LocalResource};
use leptos::prelude::ServerFnError;
use leptos::{component, view, IntoView};
use leptos_router::hooks::use_params_map;
use logmancer_core::PageResult;

#[component]
pub fn LogView() -> impl IntoView {
    let params = use_params_map();
    let file_id = move || params.get().get("id").unwrap_or_default();
    let (start_line, set_start_line) = signal(0usize);
    let (page_size, _) = signal(30usize);

    let log_page = LocalResource::new(
        move || fetch_page(file_id(), start_line.get(), page_size.get()));

    provide_context(LogViewContext {
        file_id: file_id(),
        start_line,
        set_start_line,
        page_size,
        log_page
    });

    view! {
        <MainPane />
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