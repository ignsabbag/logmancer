use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct OpenServerFileRequest {
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OpenServerFileResponse {
    pub file_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerBrowserStatusResponse {
    pub enabled: bool,
    pub message: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerBrowserListRequest {
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerBrowserListResponse {
    pub current_path: String,
    pub can_go_up: bool,
    pub entries: Vec<ServerBrowserEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ServerBrowserEntry {
    pub name: String,
    pub path: String,
    pub entry_type: String,
    pub size: Option<u64>,
    pub modified: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerBrowserOpenRequest {
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FileInfoRequest {
    pub file_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReadPageRequest {
    pub file_id: String,
    pub start_line: usize,
    pub max_lines: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TailRequest {
    pub file_id: String,
    pub max_lines: usize,
    pub follow: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApplyFilterRequest {
    pub file_id: String,
    pub filter: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReadFilterRequest {
    pub file_id: String,
    pub start_line: usize,
    pub max_lines: usize,
}
