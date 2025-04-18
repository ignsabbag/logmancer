pub struct PageResult {
    pub lines: Vec<String>,
    pub start_line: usize,
    pub total_lines: usize,
    pub indexing_progress: f64
}

impl PartialEq for PageResult {
    fn eq(&self, other: &Self) -> bool {
        self.start_line == other.start_line &&
            self.total_lines == other.total_lines
    }
}

impl Eq for PageResult {}