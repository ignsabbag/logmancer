use leptos::prelude::{window, ServerFnError};
use logmancer_core::{FileInfo, PageResult};
use crate::api::commons::{FileInfoRequest, ReadPageRequest, TailRequest};

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

pub async fn fetch_info(file_id: String) -> Result<FileInfo, ServerFnError> {
    let base = window().location().origin().unwrap();
    let url = format!("{}/api/file_info", base);
    let request = reqwest::Client::new()
        .get(url)
        .query(&FileInfoRequest {
            file_id
        });
    let result = request
        .send()
        .await?
        .json::<FileInfo>()
        .await?;
    Ok(result)
}