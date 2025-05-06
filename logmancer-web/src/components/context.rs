use leptos::prelude::{LocalResource, ReadSignal, ServerFnError, WriteSignal};
use logmancer_core::PageResult;

#[derive(Clone)]
pub struct LogViewContext {
    pub file_id: String,
    pub start_line: ReadSignal<usize>,
    pub set_start_line: WriteSignal<usize>,
    pub page_size: ReadSignal<usize>,
    pub log_page: LocalResource<Result<PageResult,ServerFnError>>
}