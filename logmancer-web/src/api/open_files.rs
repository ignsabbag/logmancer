use dashmap::DashMap;
use logmancer_core::LogReader;
use std::sync::Arc;

#[derive(Clone)]
pub struct OpenFiles {
    pub readers: Arc<DashMap<String, LogReader>>,
}

impl OpenFiles {
    pub fn new() -> Self {
        Self {
            readers: Arc::new(DashMap::new())
        }
    }
}