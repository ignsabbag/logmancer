use crate::api::commons::ServerBrowserEntry;
use crate::components::async_functions::{fetch_server_browser_list, open_server_browser_file};
use leptos::html;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos_router::hooks::use_navigate;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlElement;

#[component]
pub fn ServerFileSpotlight(
    is_open: ReadSignal<bool>,
    set_is_open: WriteSignal<bool>,
) -> impl IntoView {
    let (is_loading, set_is_loading) = signal(false);
    let (entries, set_entries) = signal(Vec::<ServerBrowserEntry>::new());
    let (current_path, set_current_path) = signal(String::new());
    let (current_display_path, set_current_display_path) = signal(String::new());
    let (can_go_up, set_can_go_up) = signal(false);
    let (filter, set_filter) = signal(String::new());
    let (selected_path, set_selected_path) = signal(None::<String>);
    let (selected_is_file, set_selected_is_file) = signal(false);
    let (error_message, set_error_message) = signal(String::new());
    let navigate = use_navigate();
    let spotlight_ref: NodeRef<html::Section> = NodeRef::new();
    let filter_input_ref: NodeRef<html::Input> = NodeRef::new();

    let focus_filter = move || {
        if let Some(input) = filter_input_ref.get() {
            let _ = input.focus();
        }
    };

    let focus_first_entry = move || {
        let Some(spotlight) = spotlight_ref.get() else {
            return;
        };
        let Ok(Some(element)) = spotlight.query_selector(".server-spotlight-entry") else {
            return;
        };
        if let Ok(element) = element.dyn_into::<HtmlElement>() {
            let _ = element.focus();
        }
    };

    let load_directory = Callback::new(move |path: String| {
        set_error_message.set(String::new());
        set_is_loading.set(true);
        set_filter.set(String::new());
        set_selected_path.set(None);
        set_selected_is_file.set(false);
        focus_filter();
        let navigate_path = path.clone();
        spawn_local(async move {
            match fetch_server_browser_list(navigate_path).await {
                Ok(response) => {
                    set_entries.set(response.entries);
                    set_current_path.set(response.current_path);
                    set_current_display_path.set(response.current_display_path);
                    set_can_go_up.set(response.can_go_up);
                }
                Err(error) => {
                    set_error_message.set(error);
                }
            }
            set_is_loading.set(false);
        });
    });

    Effect::new(move |_| {
        if is_open.get() {
            load_directory.run(String::new());
        } else {
            set_filter.set(String::new());
            set_error_message.set(String::new());
            set_selected_path.set(None);
            set_selected_is_file.set(false);
        }
    });

    let open_file = Callback::new(move |path: String| {
        set_error_message.set(String::new());
        let navigate = navigate.clone();
        spawn_local(async move {
            match open_server_browser_file(path).await {
                Ok(file_id) => {
                    set_is_open.set(false);
                    navigate(&format!("/log/{file_id}"), Default::default());
                }
                Err(error) => {
                    log!("Error opening server file from spotlight: {}", error);
                    set_error_message.set(error);
                }
            }
        });
    });

    let open_selected = Callback::new(move |_| {
        let Some(path) = selected_path.get() else {
            set_error_message.set("Select a file first.".to_string());
            return;
        };

        if !selected_is_file.get() {
            set_error_message.set("Only files can be opened.".to_string());
            return;
        }

        open_file.run(path);
    });

    let go_up = move |_| {
        if !can_go_up.get() {
            return;
        }
        load_directory.run(parent_path(&current_path.get()));
    };

    let close_modal = move |_| {
        set_is_open.set(false);
    };

    let filtered_entries = Memo::new(move |_| filter_entries(&entries.get(), &filter.get()));
    let move_focus_from_filter = move |ev: leptos::ev::KeyboardEvent| {
        if !matches!(ev.key().as_str(), "ArrowDown" | "Enter") {
            return;
        }
        let entries = filtered_entries.get();
        let Some((path, is_file)) = first_focus_target(&entries) else {
            return;
        };
        ev.prevent_default();
        set_selected_path.set(Some(path));
        set_selected_is_file.set(is_file);
        focus_first_entry();
    };

    view! {
        <Show when=move || is_open.get()>
            <div class="server-spotlight-backdrop" on:click=close_modal>
                <section node_ref=spotlight_ref class="server-spotlight" on:click=move |ev| ev.stop_propagation()>
                    <header class="server-spotlight-header">
                        <h2>"Explore Server"</h2>
                        <button type="button" on:click=close_modal>
                            "Close"
                        </button>
                    </header>

                    <div class="server-spotlight-controls">
                        <button type="button" disabled=move || !can_go_up.get() on:click=go_up>
                            "Up"
                        </button>
                        <code>{move || current_display_path.get()}</code>
                    </div>

                    <input
                        node_ref=filter_input_ref
                        type="text"
                        placeholder="Filter current folder entries"
                        bind:value=(filter, set_filter)
                        on:keydown=move_focus_from_filter
                    />

                    <div class="server-spotlight-results">
                        <Show when=move || is_loading.get()>
                            <p>"Loading..."</p>
                        </Show>

                        <For
                            each=move || filtered_entries.get()
                            key=|entry| entry.path.clone()
                            let:entry
                        >
                            {move || {
                                let row_path = entry.path.clone();
                                let row_type = entry.entry_type.clone();
                                let row_name = entry.name.clone();
                                let icon_type = row_type.clone();
                                let class_path = row_path.clone();
                                let click_path = row_path.clone();
                                let click_type = row_type.clone();
                                let dblclick_path = row_path.clone();
                                let dblclick_type = row_type.clone();
                                let key_path = row_path.clone();
                                let key_type = row_type.clone();

                                view! {
                            <button
                                type="button"
                                class=move || {
                                    if selected_path.get().as_ref() == Some(&class_path) {
                                        "server-spotlight-entry is-selected"
                                    } else {
                                        "server-spotlight-entry"
                                    }
                                }
                                on:click={
                                    move |_| {
                                        set_selected_path.set(Some(click_path.clone()));
                                        set_selected_is_file.set(click_type == "file");
                                        if click_type == "directory" {
                                            load_directory.run(click_path.clone());
                                        }
                                    }
                                }
                                on:dblclick={
                                    move |_| {
                                        set_selected_path.set(Some(dblclick_path.clone()));
                                        set_selected_is_file.set(dblclick_type == "file");
                                        if dblclick_type == "file" {
                                            open_file.run(dblclick_path.clone());
                                        } else {
                                            load_directory.run(dblclick_path.clone());
                                        }
                                    }
                                }
                                on:keydown={
                                    move |ev: leptos::ev::KeyboardEvent| {
                                        if ev.key() != "Enter" {
                                            return;
                                        }
                                        ev.prevent_default();
                                        set_selected_path.set(Some(key_path.clone()));
                                        set_selected_is_file.set(key_type == "file");
                                        if key_type == "file" {
                                            open_file.run(key_path.clone());
                                        } else {
                                            load_directory.run(key_path.clone());
                                        }
                                    }
                                }
                            >
                                <span>{move || if icon_type == "directory" { "📁" } else { "📄" }}</span>
                                <span>{row_name.clone()}</span>
                            </button>
                                }
                            }}
                        </For>
                    </div>

                    <Show when=move || !error_message.get().is_empty()>
                        <p class="server-spotlight-error">{move || error_message.get()}</p>
                    </Show>

                    <footer class="server-spotlight-footer">
                        <button
                            type="button"
                            disabled=move || !selected_is_file.get()
                            on:click=move |_| open_selected.run(())
                        >
                            "Open Selected"
                        </button>
                    </footer>
                </section>
            </div>
        </Show>
    }
}

pub(crate) fn filter_entries(
    entries: &[ServerBrowserEntry],
    query: &str,
) -> Vec<ServerBrowserEntry> {
    let trimmed = query.trim().to_lowercase();
    if trimmed.is_empty() {
        return entries.to_vec();
    }

    entries
        .iter()
        .filter(|entry| entry.name.to_lowercase().contains(&trimmed))
        .cloned()
        .collect()
}

pub(crate) fn parent_path(path: &str) -> String {
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        return String::new();
    }

    let mut parts: Vec<&str> = trimmed.split('/').collect();
    parts.pop();
    parts.join("/")
}

pub(crate) fn first_focus_target(entries: &[ServerBrowserEntry]) -> Option<(String, bool)> {
    entries
        .first()
        .map(|entry| (entry.path.clone(), entry.entry_type == "file"))
}

#[cfg(test)]
mod tests {
    use super::{filter_entries, first_focus_target, parent_path};
    use crate::api::commons::ServerBrowserEntry;

    fn mk_entry(name: &str, path: &str, entry_type: &str) -> ServerBrowserEntry {
        ServerBrowserEntry {
            name: name.to_string(),
            path: path.to_string(),
            entry_type: entry_type.to_string(),
            size: None,
            modified: None,
        }
    }

    #[test]
    fn filter_entries_only_applies_to_loaded_current_directory_entries() {
        let entries = vec![
            mk_entry("app.log", "app.log", "file"),
            mk_entry("infra", "infra", "directory"),
            mk_entry("error.log", "error.log", "file"),
        ];

        let filtered = filter_entries(&entries, "log");
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].name, "app.log");
        assert_eq!(filtered[1].name, "error.log");
    }

    #[test]
    fn filter_entries_is_case_insensitive_and_returns_all_for_empty_query() {
        let entries = vec![
            mk_entry("API.LOG", "API.LOG", "file"),
            mk_entry("docs", "docs", "directory"),
        ];

        let filtered = filter_entries(&entries, "api");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "API.LOG");

        let unfiltered = filter_entries(&entries, "   ");
        assert_eq!(unfiltered.len(), 2);
    }

    #[test]
    fn parent_path_stays_at_root_and_moves_up_one_level() {
        assert_eq!(parent_path(""), "");
        assert_eq!(parent_path("service"), "");
        assert_eq!(parent_path("service/api"), "service");
    }

    #[test]
    fn first_focus_target_selects_first_entry_with_file_flag() {
        let entries = vec![
            mk_entry("app.log", "logs/app.log", "file"),
            mk_entry("archive", "logs/archive", "directory"),
        ];

        assert_eq!(
            first_focus_target(&entries),
            Some(("logs/app.log".to_string(), true))
        );
        assert_eq!(first_focus_target(&[]), None);
    }
}
