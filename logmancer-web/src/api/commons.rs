use serde::{Deserialize, Serialize};

#[derive(Serialize,Deserialize,Debug)]
pub struct OpenServerFileRequest {
    pub path: String
}

#[derive(Serialize,Deserialize,Debug)]
pub struct OpenServerFileResponse {
    pub file_id: String
}

#[derive(Serialize,Deserialize,Debug)]
pub struct ReadPageRequest {
    pub file_id: String,
    pub start_line: usize,
    pub max_lines: usize
}