use crate::api::commons::{ReadPageRequest, TailRequest};
use crate::components::context::LogViewContext;
use crate::components::index_progress_bar::IndexProgressBar;
use crate::components::main_pane::MainPane;
use leptos::prelude::ServerFnError;
use leptos::prelude::provide_context;
use leptos::prelude::{signal, window, Get, LocalResource};
use leptos::{component, view, IntoView};
use leptos_router::hooks::use_params_map;
use logmancer_core::PageResult;

#[component]
pub fn LogView() -> impl IntoView {
    let params = use_params_map();
    let file_id = params.get().get("id").unwrap_or_default();
    let (start_line, set_start_line) = signal(0_usize);
    let (page_size, set_page_size) = signal(50_usize);
    let (follow, set_follow) = signal(false);
    let (tail, set_tail) = signal(false);

    let log_page = LocalResource::new(
        move || fetch_page(file_id.clone(), start_line.get(), page_size.get(), tail.get(), follow.get()));

    provide_context(LogViewContext {
        start_line,
        set_start_line,
        page_size,
        set_page_size,
        tail,
        set_tail,
        follow,
        set_follow,
        log_page
    });

    view! {
        <MainPane />
        <IndexProgressBar />
    }
}

async fn fetch_page(file_id: String, start_line: usize, max_lines: usize, tail: bool, follow: bool) -> Result<PageResult, ServerFnError> {
    let base = window().location().origin().unwrap();
    let request = if tail {
        let url = format!("{}/api/tail", base);
        reqwest::Client::new()
            .get(url)
            .query(&TailRequest {
                file_id,
                max_lines,
                follow
            })
    } else {
        let url = format!("{}/api/read-page", base);
        reqwest::Client::new()
            .get(url)
            .query(&ReadPageRequest {
                file_id,
                start_line,
                max_lines
            })
    };
    let result = request
        .send()
        .await.map_err(|e| ServerFnError::WrappedServerError(e))?
        .json::<PageResult>()
        .await.map_err(|e| ServerFnError::WrappedServerError(e))?;
    Ok(result)
}