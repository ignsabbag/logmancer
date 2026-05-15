use crate::api::commons::{
    ApiError, ApplyFilterRequest, OpenServerFileResponse, ReadFilterRequest, ReadPageRequest,
    ServerBrowserListRequest, ServerBrowserListResponse, ServerBrowserOpenRequest,
    ServerBrowserStatusResponse, TailRequest,
};
use leptos::prelude::{window, ServerFnError};
use leptos::wasm_bindgen::{JsCast, JsValue};
use logmancer_core::PageResult;
use wasm_bindgen_futures::JsFuture;
use web_sys::{FormData, RequestInit, Response};

pub async fn fetch_page(
    file_id: String,
    start_line: usize,
    max_lines: usize,
    tail: bool,
    follow: bool,
) -> Result<PageResult, ServerFnError> {
    let base = window().location().origin().unwrap();
    let request = if tail {
        let url = format!("{base}/api/tail");
        reqwest::Client::new().get(url).query(&TailRequest {
            file_id,
            max_lines,
            follow,
        })
    } else {
        let url = format!("{base}/api/read-page");
        reqwest::Client::new().get(url).query(&ReadPageRequest {
            file_id,
            start_line,
            max_lines,
        })
    };
    let result = request.send().await?.json::<PageResult>().await?;
    Ok(result)
}

pub async fn apply_filter(file_id: String, filter: String) -> Result<String, ServerFnError> {
    let base = window().location().origin().unwrap();
    let url = format!("{base}/api/apply-filter");
    let request = reqwest::Client::new()
        .post(url)
        .json(&ApplyFilterRequest { file_id, filter });
    let result = request.send().await?.json::<String>().await?;
    Ok(result)
}

pub async fn fetch_filter_page(
    file_id: String,
    start_line: usize,
    max_lines: usize,
) -> Result<PageResult, ServerFnError> {
    let base = window().location().origin().unwrap();
    let url = format!("{base}/api/read-filter-page");
    let request = reqwest::Client::new().get(url).query(&ReadFilterRequest {
        file_id,
        start_line,
        max_lines,
    });
    let result = request.send().await?.json::<PageResult>().await?;
    Ok(result)
}

pub async fn fetch_server_browser_status() -> Result<ServerBrowserStatusResponse, String> {
    let base = window()
        .location()
        .origin()
        .map_err(|_| "Could not detect application origin.".to_string())?;
    let url = format!("{base}/api/server-browser/status");

    let response = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .map_err(|_| "Could not connect to the server.".to_string())?;

    if response.status().is_success() {
        response
            .json::<ServerBrowserStatusResponse>()
            .await
            .map_err(|_| "Could not parse server browser status.".to_string())
    } else {
        Err(parse_api_error_message(response, "Could not fetch server browser status.").await)
    }
}

pub async fn fetch_server_browser_list(path: String) -> Result<ServerBrowserListResponse, String> {
    let base = window()
        .location()
        .origin()
        .map_err(|_| "Could not detect application origin.".to_string())?;
    let url = format!("{base}/api/server-browser/list");

    let response = reqwest::Client::new()
        .post(url)
        .json(&ServerBrowserListRequest { path })
        .send()
        .await
        .map_err(|_| "Could not connect to the server.".to_string())?;

    if response.status().is_success() {
        response
            .json::<ServerBrowserListResponse>()
            .await
            .map_err(|_| "Could not parse server browser listing.".to_string())
    } else {
        Err(parse_api_error_message(response, "Could not list directory.").await)
    }
}

pub async fn open_server_browser_file(path: String) -> Result<String, String> {
    let base = window()
        .location()
        .origin()
        .map_err(|_| "Could not detect application origin.".to_string())?;
    let url = format!("{base}/api/server-browser/open");

    let response = reqwest::Client::new()
        .post(url)
        .json(&ServerBrowserOpenRequest { path })
        .send()
        .await
        .map_err(|_| "Could not connect to the server.".to_string())?;

    if response.status().is_success() {
        let payload = response
            .json::<OpenServerFileResponse>()
            .await
            .map_err(|_| "Could not parse server response.".to_string())?;
        Ok(payload.file_id)
    } else {
        Err(parse_api_error_message(response, "Could not open the requested server file.").await)
    }
}

async fn parse_api_error_message(response: reqwest::Response, fallback: &str) -> String {
    if let Ok(payload) = response.json::<ApiError>().await {
        if !payload.message.trim().is_empty() {
            return payload.message;
        }
    }

    fallback.to_string()
}

pub async fn upload_local_file(file: web_sys::File) -> Result<String, String> {
    let form_data =
        FormData::new().map_err(|_| "Could not prepare file upload data.".to_string())?;

    form_data
        .append_with_blob_and_filename("file", &file, &file.name())
        .map_err(|_| "Could not prepare the selected file for upload.".to_string())?;

    let request_init = RequestInit::new();
    request_init.set_method("POST");
    request_init.set_body(&JsValue::from(form_data));

    let base = window()
        .location()
        .origin()
        .map_err(|_| "Could not detect application origin.".to_string())?;

    let fetch_promise =
        window().fetch_with_str_and_init(&format!("{base}/api/upload-file"), &request_init);

    let fetch_response = JsFuture::from(fetch_promise)
        .await
        .map_err(|_| "Could not connect to the server.".to_string())?;

    let response: Response = fetch_response
        .dyn_into()
        .map_err(|_| "Received an invalid response from server.".to_string())?;

    let body_json = match response.json() {
        Ok(promise) => JsFuture::from(promise).await.unwrap_or(JsValue::NULL),
        Err(_) => JsValue::NULL,
    };

    if response.ok() {
        js_sys::Reflect::get(&body_json, &JsValue::from_str("file_id"))
            .ok()
            .and_then(|value| value.as_string())
            .ok_or_else(|| "Could not parse server response.".to_string())
    } else {
        let message = body_json
            .as_string()
            .unwrap_or_else(|| "Could not upload file.".to_string());

        if message.trim().is_empty() {
            Err("Could not upload file.".to_string())
        } else {
            Err(message)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ApiError;

    #[test]
    fn api_error_prefers_non_empty_message() {
        let payload = ApiError {
            code: "invalid_path".to_string(),
            message: "Invalid path token.".to_string(),
        };

        let message = if payload.message.trim().is_empty() {
            "fallback".to_string()
        } else {
            payload.message
        };

        assert_eq!(message, "Invalid path token.");
    }

    #[test]
    fn api_error_uses_fallback_for_blank_message() {
        let payload = ApiError {
            code: "open_failed".to_string(),
            message: " ".to_string(),
        };

        let message = if payload.message.trim().is_empty() {
            "fallback".to_string()
        } else {
            payload.message
        };

        assert_eq!(message, "fallback");
    }
}
