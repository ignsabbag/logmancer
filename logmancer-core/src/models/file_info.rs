use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileInfo {
    pub path: String,
    pub total_lines: usize,
    pub indexing_progress: f64
}