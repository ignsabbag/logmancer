use crate::components::context::LogViewContext;
use leptos::context::use_context;
use leptos::ev::{KeyboardEvent, WheelEvent};
use leptos::logging::log;
use leptos::prelude::*;
use leptos::{component, html, view, IntoView};
use leptos_use::use_resize_observer;
use logmancer_core::PageResult;
use std::time::Duration;

const SCROLL_RATIO: f64 = 0.2;
const DEBOUNCE_MS: u64 = 200;
const MIN_JUMP: i32 = 1;
const LINE_HEIGHT: f64 = 20.0;

const ARROW_UP: &str = "ArrowUp";
const ARROW_DOWN: &str = "ArrowDown";
const PAGE_UP: &str = "PageUp";
const PAGE_DOWN: &str = "PageDown";
const KEYS: [&str; 4] = [ARROW_DOWN, ARROW_UP, PAGE_DOWN, PAGE_UP];

#[component]
pub fn ContentLines() -> impl IntoView {
    let LogViewContext {
        set_start_line,
        page_size,
        set_page_size,
        log_page,
        ..
    } = use_context().expect("");

    let div_ref: NodeRef<html::Div> = NodeRef::new();

    let (page_result, set_page_result) = signal(None::<PageResult>);
    let (content_width, set_content_width) = signal(2048_f64);
    let (content_height, set_content_height) = signal(1080_f64);
    let (wheel_lines, set_wheel_lines) = signal(0_i32);
    let (timeout, set_timeout) = signal(None::<TimeoutHandle>);
    let (interval, set_interval) = signal(None::<IntervalHandle>);
    let (active_key, set_active_key) = signal(None::<String>);

    let process_key = move |key: &str| {
        log!("Key {} pressed", key);
        match key {
            ARROW_UP => set_start_line.update(|current| {
                *current = current.saturating_sub(MIN_JUMP as usize)
            }),
            ARROW_DOWN => set_start_line.update(|current| {
                *current = current.saturating_add(MIN_JUMP as usize)
            }),
            PAGE_UP => set_start_line.update(|current| {
                *current = current.saturating_sub(page_size.get())
            }),
            PAGE_DOWN => set_start_line.update(|current| {
                *current = current.saturating_add(page_size.get())
            }),
            "g" => set_start_line.set(0),
            _ => ()
        }
    };

    let on_key_down = move |ev: KeyboardEvent| {
        let key = ev.key();
        if KEYS.contains(&key.as_str()) {
            if active_key.get() == Some(key.clone()) {
                return;
            }
            set_active_key.set(Some(key.clone()));

            let result = set_interval_with_handle(move || {
                if let Some(key) = active_key.get() {
                    process_key(&key)
                }
            }, Duration::from_millis(DEBOUNCE_MS));

            if let Ok(handle) = result {
                set_interval.set(Some(handle));
            }
        }
    };

    let on_key_up = move |ev: KeyboardEvent| {
        let key = ev.key();
        process_key(&key);
        if KEYS.contains(&key.as_str()) {
            if let Some(handle) = interval.get() {
                handle.clear();
            }
            set_active_key.set(None);
        }
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
            if let Some(page_result) = page_result.get() {
                let handle = set_timeout_with_handle(
                    move || {
                        let delta_lines = wheel_lines.get();
                        let new_line = if delta_lines < 0 {
                            log!("Scrolling up {} lines", delta_lines);
                            page_result.start_line.saturating_sub(delta_lines.abs() as usize)
                        } else {
                            log!("Scrolling down {} lines", delta_lines);
                            page_result.start_line.saturating_add(delta_lines as usize)
                                .min(page_result.total_lines.saturating_sub(page_size.get()))
                        };
                        if page_result.start_line != new_line {
                            log!("Updating start_line by wheel to: {}", new_line);
                            set_start_line.set(new_line);
                        }
                        set_timeout.set(None);
                    },
                    Duration::from_millis(DEBOUNCE_MS)
                ).ok();
                set_timeout.set(handle);
            }
        }
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

                let mut lines = (rect.height() / LINE_HEIGHT) as usize;
                lines = lines.saturating_sub(1);
                if lines != page_size.get() {
                    log!("Updating page_size to {}", lines);
                    set_page_size.set(lines);
                }
            }
        });
    });

    view! {
        <div node_ref=div_ref on:keydown=on_key_down on:keyup=on_key_up on:wheel=on_wheel
                tabindex="0" class="content-lines">
            <Transition>
                { move || Suspend::new(async move {
                    log_page.await.map(|page_result| {
                        set_page_result.set(Some(page_result.clone()));
                        view! {
                            <ul>
                                { page_result.lines.into_iter().enumerate().map(|(i, line)| view! {
                                    <li><b>{page_result.start_line + i + 1}</b> | {line}</li>
                                }).collect::<Vec<_>>() }
                            </ul>
                        }
                    })
                })}
            </Transition>
        </div>
    }
}