use std::sync::Arc;
use crate::api::open_files::OpenFiles;

#[derive(Clone)]
pub struct AppState {
    pub open_files: Arc<OpenFiles>
}