use leptos::prelude::{window, ServerFnError};
use logmancer_core::PageResult;
use crate::api::commons::{ApplyFilterRequest, ReadFilterRequest, ReadPageRequest, TailRequest};

pub async fn fetch_page(file_id: String, start_line: usize, max_lines: usize, tail: bool, follow: bool) -> Result<PageResult, ServerFnError> {
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
        .await?
        .json::<PageResult>()
        .await?;
    Ok(result)
}

pub async fn apply_filter(file_id: String, filter: String) -> Result<String, ServerFnError> {
    let base = window().location().origin().unwrap();
    let url = format!("{}/api/apply-filter", base);
    let request = reqwest::Client::new()
        .post(url)
        .json(&ApplyFilterRequest {
            file_id,
            filter
        });
    let result = request
        .send()
        .await?
        .json::<String>()
        .await?;
    Ok(result)
}

pub async fn fetch_filter_page(file_id: String, start_line: usize, max_lines: usize) -> Result<PageResult, ServerFnError> {
    let base = window().location().origin().unwrap();
    let url = format!("{}/api/read-filter-page", base);
    let request = reqwest::Client::new()
        .get(url)
        .query(&ReadFilterRequest {
            file_id,
            start_line,
            max_lines
        });
    let result = request
        .send()
        .await?
        .json::<PageResult>()
        .await?;
    Ok(result)
}
