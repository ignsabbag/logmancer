use leptos::html;
use leptos::prelude::*;

#[component]
pub fn AppBar(
    #[prop(into)] path: Signal<String>,
    open_visual_rules: Callback<()>,
    visual_rules_button_ref: NodeRef<html::Button>,
) -> impl IntoView {
    view! {
        <header class="app-bar">
            <span class="app-bar__spacer"></span>
            <span
                class="app-bar__filename"
                title=move || path.get()
                aria-label=move || format!("Open file: {}", path.get())
            >
                {move || path.get()}
            </span>
            <div class="app-bar__actions">
                <button node_ref=visual_rules_button_ref type="button" on:click=move |_| open_visual_rules.run(())>"Visual Rules"</button>
                <button type="button" aria-label="Future actions" title="Future actions">"…"</button>
            </div>
        </header>
    }
}
