use crate::components::context::{LogFileContext, LogViewContext};
use leptos::context::use_context;
use leptos::ev::{KeyboardEvent, WheelEvent};
use leptos::logging::log;
use leptos::prelude::*;
use leptos::{component, view, IntoView};
use logmancer_core::PageResult;
use std::time::Duration;

const SCROLL_RATIO: f64 = 0.3;
const DEBOUNCE_MS: u64 = 200;
const MIN_JUMP: i32 = 3;

const ARROW_UP: &str = "ArrowUp";
const ARROW_DOWN: &str = "ArrowDown";
const PAGE_UP: &str = "PageUp";
const PAGE_DOWN: &str = "PageDown";
const KEYS: [&str; 4] = [ARROW_DOWN, ARROW_UP, PAGE_DOWN, PAGE_UP];

#[component]
pub fn ContentLines(context: LogViewContext) -> impl IntoView {
    let LogFileContext {
        tail,
        set_tail,
        follow,
        set_follow,
        ..
    } = use_context().expect("LogFileContext not found");
    
    let LogViewContext {
        set_start_line,
        page_size,
        set_page_size,
        log_page
    } = context;

    let (wheel_lines, set_wheel_lines) = signal(0_i32);
    let (debounce, set_debounce) = signal(None::<TimeoutHandle>);
    let (interval, set_interval) = signal(None::<IntervalHandle>);
    let (active_key, set_active_key) = signal(None::<String>);

    let (page_result, set_page_result) = signal(None::<PageResult>);

    let update_tail = move |new_line: usize| {
        set_tail.update_untracked(move |current| {
            let page_result = page_result.get().unwrap();
            if page_result.start_line > new_line {
                log!("Updating tail to false");
                *current = false
            } else if new_line.saturating_add(page_size.get()) >= page_result.total_lines {
                log!("Updating tail to true");
                *current = true
            }
        });
    };

    let process_key = move |key: &str| {
        let page_result = page_result.get().unwrap();
        log!("Key {} pressed", key);
        match key {
            ARROW_UP => {
                let new_line = page_result.start_line.saturating_sub(MIN_JUMP as usize);
                update_tail(new_line);
                set_start_line.set(new_line);
            },
            ARROW_DOWN => {
                let new_line = page_result.start_line.saturating_add(MIN_JUMP as usize);
                update_tail(new_line);
                set_start_line.set(new_line);
            },
            PAGE_UP => {
                let new_line = page_result.start_line.saturating_sub(page_size.get());
                update_tail(new_line);
                set_start_line.set(new_line);
            },
            PAGE_DOWN => {
                let new_line = page_result.start_line.saturating_add(page_size.get());
                update_tail(new_line);
                set_start_line.set(new_line);
            },
            "g" => {
                update_tail(0);
                set_start_line.set(0);
            },
            "G" => set_tail.set(true),
            "f"|"F" => set_follow.update(|current| *current = !current.to_owned()),
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

        if let None = debounce.get() {
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
                            set_tail.update_untracked(move |current| {
                                if page_result.start_line > new_line {
                                    *current = false
                                } else if new_line + page_size.get() > page_result.total_lines {
                                    *current = true
                                }
                            });
                            set_start_line.set(new_line);
                        }
                        set_debounce.set(None);
                    },
                    Duration::from_millis(DEBOUNCE_MS)
                ).ok();
                set_debounce.set(handle);
            }
        }
    };

    view! {
        <Transition>
            { move || Suspend::new(async move {
                log_page.await.map(|page_result| {
                    set_page_result.set(Some(page_result.clone()));
                    if tail.get() && follow.get() {
                        set_timeout(move || set_page_size.notify(), Duration::from_secs(1));
                    } else {
                        update_tail(page_result.start_line);
                    }
                    view! {
                        <div class="line-numbers">
                            { (0..page_result.lines.len()).map(|i| view! {
                                <div><b>{page_result.start_line + i + 1}</b></div>
                            }).collect::<Vec<_>>() }
                        </div>
                        <div
                            class="text-lines" tabindex="0"
                            on:keydown=on_key_down on:keyup=on_key_up
                            on:wheel=on_wheel
                        >
                            { page_result.lines.into_iter().map(|line| view! {
                                <div>{line}</div>
                            }).collect::<Vec<_>>() }
                        </div>
                    }
                })
            })}
        </Transition>
    }
}