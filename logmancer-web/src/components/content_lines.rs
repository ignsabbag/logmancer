use crate::components::context::LogViewContext;
use leptos::context::use_context;
use leptos::ev::{KeyboardEvent, WheelEvent};
use leptos::logging::log;
use leptos::prelude::{set_timeout_with_handle, signal, ClassAttribute, Effect, ElementChild, Get, NodeRef, NodeRefAttribute, OnAttribute, Set, Suspend, TimeoutHandle, Transition};
use leptos::{component, html, view, IntoView};
use std::time::Duration;

const SCROLL_RATIO: f64 = 0.2;
const DEBOUNCE_MS: u64 = 200;
const MIN_JUMP: i32 = 1;

#[component]
pub fn ContentLines() -> impl IntoView {
    let LogViewContext {
        start_line,
        set_start_line,
        total_lines,
        page_size,
        set_page_size,
        log_page,
        ..
    } = use_context().expect("");

    let div_ref: NodeRef<html::Div> = NodeRef::new();

    let (wheel_lines, set_wheel_lines) = signal(0_i32);
    let (timeout, set_timeout) = signal::<Option<TimeoutHandle>>(None);


    let on_key_down = move |ev: KeyboardEvent| {/*
        ev.prevent_default();
        match ev.key().as_str() {
            "ArrowUp" => {
                log!("ArrowUp");
                set_start_line.update(|current| *current -= MIN_JUMP)
            },
            "ArrowDown" => {
                log!("ArrowDown");
                set_start_line.update(|current| *current += MIN_JUMP)
            },
            "PageUp" => {
                log!("PageUp");
                set_start_line.update(|current| *current += page_size.get())
            },
            "PageDown" => {
                log!("PageDown");
                set_start_line.update(|current| *current += page_size.get())
            },
            _ => ()
        }*/
    };

    let on_wheel = move |ev: WheelEvent| {
        ev.prevent_default();

        let delta = ev.delta_y().abs();
        let signum = ev.delta_y().signum() as i32;
        let is_precise_scroll = ev.delta_mode() == 0;

        log!("Wheel detected: {}", delta);
        let lines_to_jump = if is_precise_scroll {
            let lines = delta * SCROLL_RATIO;
            MIN_JUMP.max(lines as i32)
        } else {
            MIN_JUMP.max((page_size.get() as f64 * SCROLL_RATIO) as i32)
        };
        set_wheel_lines.set(lines_to_jump * signum);

        if let None = timeout.get() {
            let handle = set_timeout_with_handle(
                move || {
                    let delta_lines = wheel_lines.get();
                    let new_line = if delta_lines < 0 {
                        log!("Scrolling up {} lines", delta_lines);
                        start_line.get().saturating_sub(delta_lines.abs() as usize)
                    } else {
                        log!("Scrolling down {} lines", delta_lines);
                        start_line.get().saturating_add(delta_lines as usize)
                            .min(total_lines.get().saturating_sub(page_size.get()))
                    };
                    if start_line.get() != new_line {
                        log!("Updating start_line by wheel to: {}", new_line);
                        set_start_line.set(new_line);
                    }
                    set_timeout.set(None);
                },
                Duration::from_millis(DEBOUNCE_MS)
            ).ok();
            set_timeout.set(handle);
        }
    };

    Effect::new(move || {
        if let Some(div) = div_ref.get() {
            let mut lines = (div.client_height() as f32 / 20.0) as usize;
            lines = lines.saturating_sub(1);
            if lines != page_size.get() {
                set_page_size.set(lines);
            }
        }
    });
    
    view! {
        <div node_ref=div_ref on:keydown=on_key_down on:wheel=on_wheel class="content-lines">
            <Transition fallback=move || view! { <p>"Loading..."</p> }>
                <ul>
                    { move || Suspend::new(async move {
                        log_page.await.map(|page_result| view! {
                            { page_result.lines.into_iter().enumerate().map(|(i, line)| view! {
                                <li><b>{page_result.start_line + i + 1}</b> | {line}</li>
                            }).collect::<Vec<_>>() }
                        })
                    })}
                </ul>
            </Transition>
        </div>
    }
}