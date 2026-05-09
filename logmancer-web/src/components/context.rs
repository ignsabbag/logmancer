use leptos::prelude::{LocalResource, Memo, ReadSignal, ServerFnError, WriteSignal};
use logmancer_core::PageResult;

#[derive(Clone)]
pub struct Port(pub u16);

#[derive(Clone)]
pub struct LogFileContext {
    pub file_id: Memo<String>,
    pub follow: ReadSignal<bool>,
    pub set_follow: WriteSignal<bool>,
    pub tail: ReadSignal<bool>,
    pub set_tail: WriteSignal<bool>,
}

#[derive(Clone)]
pub struct SelectionContext {
    pub selected_original_line: ReadSignal<Option<usize>>,
    pub set_selected_original_line: WriteSignal<Option<usize>>,
    pub selected_line_source: ReadSignal<SelectionSource>,
    pub set_selected_line_source: WriteSignal<SelectionSource>,
}

#[derive(Clone)]
pub struct ActivePaneContext {
    pub active_pane: ReadSignal<SelectionSource>,
    pub set_active_pane: WriteSignal<SelectionSource>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SelectionSource {
    Main,
    Filter,
}

#[derive(Clone)]
pub struct LogViewContext {
    pub set_start_line: WriteSignal<usize>,
    pub page_size: ReadSignal<usize>,
    pub set_page_size: WriteSignal<usize>,
    pub log_page: LocalResource<Result<PageResult, ServerFnError>>,
    pub indexing_progress: ReadSignal<f64>,
    pub set_indexing_progress: WriteSignal<f64>,
    pub selected_line: ReadSignal<Option<usize>>,
    pub set_selected_line: WriteSignal<Option<usize>>,
    pub selection_source: SelectionSource,
    pub set_selected_line_source: WriteSignal<SelectionSource>,
    pub set_active_pane: WriteSignal<SelectionSource>,
}
