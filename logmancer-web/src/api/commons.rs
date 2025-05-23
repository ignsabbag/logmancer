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

#[derive(Serialize,Deserialize,Debug)]
pub struct TailRequest {
    pub file_id: String,
    pub max_lines: usize,
    pub follow: bool
}