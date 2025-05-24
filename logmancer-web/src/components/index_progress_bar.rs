use std::path::Path;
use crate::components::context::LogFileContext;
use leptos::context::use_context;
use leptos::prelude::*;
use std::time::Duration;
use leptos::logging::log;

#[component]
pub fn IndexProgressBar() -> impl IntoView {
    let LogFileContext {
        set_indexing_progress,
        follow,
        tail,
        log_info,
        ..
    } = use_context().expect("");
    
    view! {
        <Transition>
            { move || Suspend::new(async move {
                log_info.await.map(|file_info| {
                    log!("{:?}", file_info);
                    let path = Path::new(&file_info.path)
                        .file_name().unwrap().to_str().unwrap();
                    if file_info.indexing_progress < 1.0 || (tail.get() && follow.get()) {
                        set_timeout(move || set_indexing_progress.set(file_info.indexing_progress), Duration::from_secs(1));
                        let indexing = file_info.indexing_progress * 100.0;
                        document().set_title(&format!("{:.2}% - {}", indexing, path));
                    } else {
                        document().set_title(&path);
                    }
                    view! {
                        <div
                            id="progress-bar"
                            class:hidden=move || { file_info.indexing_progress >= 1.0 }
                            style:width=move || { format!("{}%", file_info.indexing_progress * 100.0) }
                        ></div>
                    }
                })
            })}
        </Transition>
    }
}
