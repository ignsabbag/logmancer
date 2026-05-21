use crate::components::context::{SearchCommandContext, SearchUiContext};
use leptos::ev::KeyboardEvent;
use leptos::html::Input;
use leptos::prelude::*;
use leptos::{component, view, IntoView};

#[component]
pub fn SearchPanel() -> impl IntoView {
    let SearchUiContext {
        visible,
        query,
        set_query,
        status,
        set_status,
        focus_request,
        request_focus: request_search_focus,
        request_close,
        ..
    } = use_context().expect("SearchUiContext not found");
    let SearchCommandContext {
        request_submit,
        request_clear,
        ..
    } = use_context().expect("SearchCommandContext not found");
    let input_ref = NodeRef::<Input>::new();

    let close_panel = move || request_close.update(|request| *request = request.saturating_add(1));

    let on_key_down = move |ev: KeyboardEvent| match ev.key().as_str() {
        "Enter" => {
            ev.prevent_default();
            ev.stop_propagation();
            // Empty submissions are explicit clear commands; MainPane owns backend state changes.
            if query.get().trim().is_empty() {
                request_clear.update(|request| *request = request.saturating_add(1));
            } else {
                request_submit.update(|request| *request = request.saturating_add(1));
            }
        }
        "Escape" => {
            ev.prevent_default();
            ev.stop_propagation();
            close_panel();
        }
        _ => (),
    };

    let on_input = move |ev| {
        set_query.set(event_target_value(&ev));
        set_status.set(String::new());
    };

    Effect::new(move || {
        if !visible.get() {
            return;
        }
        let _ = focus_request.get();

        if let Some(input) = input_ref.get() {
            request_animation_frame(move || {
                _ = input.focus();
                input.select();
            });
        }
    });

    view! {
        <div
            class="search-panel"
            class:search-panel--hidden=move || !visible.get()
            role="search"
            aria-hidden=move || (!visible.get()).to_string()
        >
            <span
                class="search-panel__prefix"
                on:click=move |_| {
                    request_search_focus.update(|request| *request = request.saturating_add(1));
                }
            >"/"</span>
            <input
                node_ref=input_ref
                type="text"
                class="search-panel__input"
                placeholder="Search logs"
                value=query
                tabindex=move || if visible.get() { "0" } else { "-1" }
                on:input=on_input
                on:keydown=on_key_down
            />
            <span class="search-panel__status" aria-live="polite">{move || status.get()}</span>
            <button
                type="button"
                class="search-panel__close"
                aria-label="Close search"
                tabindex=move || if visible.get() { "0" } else { "-1" }
                on:click=move |_| close_panel()
            >"x"</button>
        </div>
    }
}
