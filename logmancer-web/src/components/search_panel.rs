use leptos::ev::KeyboardEvent;
use leptos::html::Input;
use leptos::prelude::*;
use leptos::{component, view, IntoView};

#[component]
pub fn SearchPanel(
    visible: ReadSignal<bool>,
    text: ReadSignal<String>,
    set_text: WriteSignal<String>,
    status: ReadSignal<String>,
    set_status: WriteSignal<String>,
    focus_request: ReadSignal<u64>,
    on_submit: impl Fn() + Copy + Send + Sync + 'static,
    on_close: impl Fn() + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let input_ref = NodeRef::<Input>::new();

    let on_key_down = move |ev: KeyboardEvent| match ev.key().as_str() {
        "Enter" => {
            ev.prevent_default();
            on_submit();
        }
        "Escape" => {
            ev.prevent_default();
            on_close();
        }
        _ => (),
    };

    let on_input = move |ev| {
        set_text.set(event_target_value(&ev));
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
        <Show when=move || visible.get()>
            <div class="search-panel" role="search">
                <span class="search-panel__prefix">"/"</span>
                <input
                    node_ref=input_ref
                    type="text"
                    class="search-panel__input"
                    placeholder="Search logs"
                    value=text
                    on:input=on_input
                    on:keydown=on_key_down
                />
                <span class="search-panel__status" aria-live="polite">{move || status.get()}</span>
                <button
                    type="button"
                    class="search-panel__close"
                    aria-label="Close search"
                    on:click=move |_| on_close()
                >"x"</button>
            </div>
        </Show>
    }
}
