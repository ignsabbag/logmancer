use serde::{Deserialize, Serialize};

use crate::models::search::PageSearchResult;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PageLine {
    pub number: usize,
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PageResult {
    pub lines: Vec<PageLine>,
    pub start_line: usize,
    pub total_lines: usize,
    pub indexing_progress: f64,
    pub search: Option<PageSearchResult>,
}

impl PartialEq for PageResult {
    fn eq(&self, other: &Self) -> bool {
        self.lines == other.lines
            && self.start_line == other.start_line
            && self.total_lines == other.total_lines
            && self.indexing_progress == other.indexing_progress
            && self.search == other.search
    }
}

impl Eq for PageResult {}
