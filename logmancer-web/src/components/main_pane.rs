use crate::components::async_functions::fetch_page;
use crate::components::content_lines::ContentLines;
use crate::components::content_scroll::ContentScroll;
use crate::components::context::{LogFileContext, LogViewContext};
use leptos::context::use_context;
use leptos::leptos_dom::log;
use leptos::prelude::{signal, ClassAttribute, Effect, ElementChild, Get, LocalResource, NodeRef, NodeRefAttribute, Set};
use leptos::{component, html, view, IntoView};
use leptos_use::use_resize_observer;

const LINE_HEIGHT: f64 = 15.0;

#[component]
pub fn MainPane() -> impl IntoView {
    let LogFileContext {
        file_id,
        tail,
        follow,
        ..
    } = use_context().expect("");

    let div_ref: NodeRef<html::Div> = NodeRef::new();
    let (content_width, set_content_width) = signal(2048_f64);
    let (content_height, set_content_height) = signal(1080_f64);

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

    Effect::new(move || {
        use_resize_observer(div_ref, move |entries, _observer| {
            let rect = entries[0].content_rect();
            if content_width.get() != rect.width() {
                log!("Updating content width to {}", rect.width());
                set_content_width.set(rect.width());
            }
            if content_height.get() != rect.height() {
                log!("Updating content height to {}", rect.height());
                set_content_height.set(rect.height());

                let lines = (rect.height() / LINE_HEIGHT) as usize;
                if lines != page_size.get() {
                    log!("Updating page_size to {}", lines);
                    set_page_size.set(lines);
                }
            }
        });
    });
    
    view! {
        <div class="main-pane">
            <div node_ref=div_ref class="content">
                <ContentLines context=log_view_context.clone() />
                <ContentScroll context=log_view_context.clone() />
            </div>
        </div>
    }
}