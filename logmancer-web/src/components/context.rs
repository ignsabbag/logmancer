use leptos::prelude::{LocalResource, ReadSignal, ServerFnError, WriteSignal};
use logmancer_core::PageResult;

#[derive(Clone)]
pub struct LogViewContext {
    pub start_line: ReadSignal<usize>,
    pub set_start_line: WriteSignal<usize>,
    pub page_size: ReadSignal<usize>,
    pub set_page_size: WriteSignal<usize>,
    pub tail: ReadSignal<bool>,
    pub set_tail: WriteSignal<bool>,
    pub follow: ReadSignal<bool>,
    pub set_follow: WriteSignal<bool>,
    pub log_page: LocalResource<Result<PageResult,ServerFnError>>
}