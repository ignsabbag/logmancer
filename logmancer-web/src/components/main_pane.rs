use crate::components::content_lines::ContentLines;
use crate::components::content_scroll::ContentScroll;
use crate::components::context::{LogFileContext, LogViewContext};
use crate::components::async_functions::fetch_page;
use leptos::context::use_context;
use leptos::prelude::{signal, ClassAttribute, ElementChild, Get, LocalResource};
use leptos::{component, view, IntoView};

#[component]
pub fn MainPane() -> impl IntoView {
    let LogFileContext {
        file_id,
        tail,
        follow,
        ..
    } = use_context().expect("");

    let (start_line, set_start_line) = signal(0_usize);
    let (page_size, set_page_size) = signal(50_usize);
    
    let log_page = LocalResource::new(move ||
        fetch_page(file_id.get(), start_line.get(), page_size.get(), tail.get(), follow.get()));
    
    let log_view_context = LogViewContext {
        set_start_line,
        page_size,
        set_page_size,
        log_page
    };
    
    view! {
        <div class="main-pane">
            <div class="content">
                <ContentLines context=log_view_context.clone() />
                <ContentScroll context=log_view_context.clone() />
            </div>
        </div>
    }
}