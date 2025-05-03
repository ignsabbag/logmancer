use std::sync::Arc;
use logmancer_core::LogRegistry;

#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<LogRegistry>
}