use leptos::prelude::{LocalResource, Memo, ReadSignal, ServerFnError, WriteSignal};
use logmancer_core::{FileInfo, PageResult};

#[derive(Clone)]
pub struct Port(pub u16);

#[derive(Clone)]
pub struct LogFileContext {
    pub file_id: Memo<String>,
    pub set_indexing_progress: WriteSignal<f64>,
    pub follow: ReadSignal<bool>,
    pub set_follow: WriteSignal<bool>,
    pub tail: ReadSignal<bool>,
    pub set_tail: WriteSignal<bool>,
    pub log_info: LocalResource<Result<FileInfo,ServerFnError>>
}

#[derive(Clone)]
pub struct LogViewContext {
    pub set_start_line: WriteSignal<usize>,
    pub page_size: ReadSignal<usize>,
    pub set_page_size: WriteSignal<usize>,
    pub log_page: LocalResource<Result<PageResult,ServerFnError>>
}