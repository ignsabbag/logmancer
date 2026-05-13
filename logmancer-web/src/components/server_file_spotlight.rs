use crate::api::commons::ServerBrowserEntry;
use crate::components::async_functions::{fetch_server_browser_list, open_server_browser_file};
use leptos::logging::log;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn ServerFileSpotlight(
    is_open: ReadSignal<bool>,
    set_is_open: WriteSignal<bool>,
) -> impl IntoView {
    let (is_loading, set_is_loading) = signal(false);
    let (entries, set_entries) = signal(Vec::<ServerBrowserEntry>::new());
    let (current_path, set_current_path) = signal(String::new());
    let (can_go_up, set_can_go_up) = signal(false);
    let (filter, set_filter) = signal(String::new());
    let (selected_path, set_selected_path) = signal(None::<String>);
    let (selected_is_file, set_selected_is_file) = signal(false);
    let (error_message, set_error_message) = signal(String::new());
    let navigate = use_navigate();

    let load_directory = Callback::new(move |path: String| {
        set_error_message.set(String::new());
        set_is_loading.set(true);
        set_selected_path.set(None);
        set_selected_is_file.set(false);
        let navigate_path = path.clone();
        spawn_local(async move {
            match fetch_server_browser_list(navigate_path).await {
                Ok(response) => {
                    set_entries.set(response.entries);
                    set_current_path.set(response.current_path);
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

    let open_selected = Callback::new(move |_| {
        let Some(path) = selected_path.get() else {
            set_error_message.set("Seleccioná un archivo primero.".to_string());
            return;
        };

        if !selected_is_file.get() {
            set_error_message.set("Solo podés abrir archivos.".to_string());
            return;
        }

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

    view! {
        <Show when=move || is_open.get()>
            <div class="server-spotlight-backdrop" on:click=close_modal>
                <section class="server-spotlight" on:click=move |ev| ev.stop_propagation()>
                    <header class="server-spotlight-header">
                        <h2>"Explorar servidor"</h2>
                        <button type="button" on:click=close_modal>
                            "Cerrar"
                        </button>
                    </header>

                    <div class="server-spotlight-controls">
                        <button type="button" disabled=move || !can_go_up.get() on:click=go_up>
                            "Subir"
                        </button>
                        <code>{move || {
                            let path = current_path.get();
                            if path.is_empty() {
                                "/".to_string()
                            } else {
                                format!("/{path}")
                            }
                        }}</code>
                    </div>

                    <input
                        type="text"
                        placeholder="Filtrar entradas del directorio actual"
                        bind:value=(filter, set_filter)
                    />

                    <div class="server-spotlight-results" on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                        if ev.key() == "Enter" {
                            open_selected.run(());
                        }
                    }>
                        <Show when=move || is_loading.get()>
                            <p>"Cargando..."</p>
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
                                            open_selected.run(());
                                        } else {
                                            load_directory.run(dblclick_path.clone());
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
                        <p class="home-error">{move || error_message.get()}</p>
                    </Show>

                    <footer class="server-spotlight-footer">
                        <button
                            type="button"
                            disabled=move || !selected_is_file.get()
                            on:click=move |_| open_selected.run(())
                        >
                            "Abrir seleccionado"
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

#[cfg(test)]
mod tests {
    use super::{filter_entries, parent_path};
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
}
