use crate::api::commons::{FileInfoRequest, ReadPageRequest, TailRequest};
use crate::components::context::LogViewContext;
use crate::components::index_progress_bar::IndexProgressBar;
use crate::components::main_pane::MainPane;
use leptos::prelude::ServerFnError;
use leptos::prelude::provide_context;
use leptos::prelude::{signal, window, Get, LocalResource};
use leptos::{component, view, IntoView};
use leptos_router::hooks::use_params_map;
use logmancer_core::{FileInfo, PageResult};

#[component]
pub fn LogView() -> impl IntoView {
    let file_id = move || {
        use_params_map().get().get("id").unwrap_or_default()
    };
    let (indexing_progress, set_indexing_progress) = signal(0_f64);
    let (start_line, set_start_line) = signal(0_usize);
    let (page_size, set_page_size) = signal(50_usize);
    let (follow, set_follow) = signal(false);
    let (tail, set_tail) = signal(false);

    let log_info = LocalResource::new(move || {
        indexing_progress.get();
        file_info(file_id())
    });
    let log_page = LocalResource::new(move || 
        fetch_page(file_id(), start_line.get(), page_size.get(), tail.get(), follow.get()));

    provide_context(LogViewContext {
        set_start_line,
        page_size,
        set_page_size,
        tail,
        set_tail,
        follow,
        set_follow,
        log_info,
        log_page
    });

    view! {
        <MainPane />
        <IndexProgressBar set_index=set_indexing_progress />
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

async fn file_info(file_id: String) -> Result<FileInfo, ServerFnError> {
    let base = window().location().origin().unwrap();
    let url = format!("{}/api/file_info", base);
    let request = reqwest::Client::new()
        .get(url)
        .query(&FileInfoRequest {
            file_id
        });
    let result = request
        .send()
        .await.map_err(|e| ServerFnError::WrappedServerError(e))?
        .json::<FileInfo>()
        .await.map_err(|e| ServerFnError::WrappedServerError(e))?;
    Ok(result)
}