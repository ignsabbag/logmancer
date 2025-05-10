use leptos::prelude::{LocalResource, ReadSignal, ServerFnError, WriteSignal};
use logmancer_core::PageResult;

#[derive(Clone)]
pub struct LogViewContext {
    pub file_id: String,
    pub start_line: ReadSignal<usize>,
    pub set_start_line: WriteSignal<usize>,
    pub total_lines: ReadSignal<usize>,
    pub set_total_lines: WriteSignal<usize>,
    pub page_size: ReadSignal<usize>,
    pub set_page_size: WriteSignal<usize>,
    pub log_page: LocalResource<Result<PageResult,ServerFnError>>
}