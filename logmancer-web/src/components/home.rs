use crate::components::async_functions::{open_server_file, upload_local_file};
use leptos::logging::log;
use leptos::prelude::*;
use leptos::*;
use leptos_router::hooks::use_navigate;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{DragEvent, Event, File, HtmlInputElement};

#[component]
pub fn Home() -> impl IntoView {
    let (server_path, set_server_path) = signal(String::new());
    let (open_error, set_open_error) = signal(String::new());
    let (upload_error, set_upload_error) = signal(String::new());
    let (is_loading_server, set_is_loading_server) = signal(false);
    let (is_uploading, set_is_uploading) = signal(false);
    let (is_dragging, set_is_dragging) = signal(false);
    let navigate = use_navigate();
    let navigate_for_upload = navigate.clone();

    let upload_file = Callback::new(move |file: File| {
        set_upload_error.set(String::new());
        set_is_uploading.set(true);
        let navigate = navigate_for_upload.clone();

        spawn_local(async move {
            match upload_local_file(file).await {
                Ok(file_id) => {
                    navigate(&format!("/log/{file_id}"), Default::default());
                }
                Err(err) => {
                    log!("Error uploading file: {}", err);
                    set_upload_error.set(err);
                }
            }

            set_is_uploading.set(false);
        });
    });

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let path_value = server_path.get();
        let trimmed_path = path_value.trim().to_string();
        if trimmed_path.is_empty() {
            set_open_error.set("Please enter a valid path.".to_string());
            return;
        }

        set_open_error.set(String::new());
        set_is_loading_server.set(true);
        let navigate = navigate.clone();
        spawn_local(async move {
            match open_server_file(trimmed_path).await {
                Ok(file_id) => {
                    navigate(&format!("/log/{file_id}"), Default::default());
                }
                Err(err) => {
                    log!("Error opening server file: {}", err);
                    set_open_error.set(err);
                }
            }
            set_is_loading_server.set(false);
        });
    };

    let on_file_change = move |ev: Event| {
        if is_uploading.get_untracked() {
            return;
        }

        let Some(target) = ev.target() else {
            return;
        };

        let Ok(input) = target.dyn_into::<HtmlInputElement>() else {
            return;
        };

        if let Some(file) = input.files().and_then(|files| files.get(0)) {
            upload_file.run(file);
            input.set_value("");
        }
    };

    let on_drag_over = move |ev: DragEvent| {
        ev.prevent_default();
        set_is_dragging.set(true);
    };

    let on_drag_leave = move |ev: DragEvent| {
        ev.prevent_default();
        set_is_dragging.set(false);
    };

    let on_drop = move |ev: DragEvent| {
        ev.prevent_default();
        set_is_dragging.set(false);

        if let Some(file) = ev
            .data_transfer()
            .and_then(|dt| dt.files())
            .and_then(|files| files.get(0))
        {
            upload_file.run(file);
        }
    };

    view! {
        <main class="home-landing">
            <section class="home-card">
                <h1>"Logmancer"</h1>
                <p class="home-subtitle">"Explorá logs grandes desde el navegador, sin vueltas."</p>

                <div
                    class=move || {
                        if is_dragging.get() {
                            "home-dropzone is-dragging"
                        } else {
                            "home-dropzone"
                        }
                    }
                    on:dragover=on_drag_over
                    on:dragleave=on_drag_leave
                    on:drop=on_drop
                >
                    <p class="home-dropzone-title">"Arrastrá y soltá un archivo local"</p>
                    <p class="home-dropzone-subtitle">"o elegilo manualmente para subirlo"</p>

                    <input
                        id="home-local-file-input"
                        class="home-file-input"
                        type="file"
                        on:change=on_file_change
                    />

                    <label
                        for="home-local-file-input"
                        class=move || {
                            if is_uploading.get() {
                                "home-upload-btn is-disabled"
                            } else {
                                "home-upload-btn"
                            }
                        }
                    >
                        {move || if is_uploading.get() { "Subiendo..." } else { "Elegir archivo local" }}
                    </label>
                </div>

                <Show when=move || !upload_error.get().is_empty()>
                    <p class="home-error">{move || upload_error.get()}</p>
                </Show>

                <div class="home-divider">
                    <span>"o abrir desde el servidor"</span>
                </div>

                <form class="home-server-form" on:submit=on_submit>
                    <input
                        type="text"
                        placeholder="Ruta en el servidor"
                        bind:value=(server_path, set_server_path)
                    />
                    <button type="submit" disabled=move || is_loading_server.get()>
                        {move || if is_loading_server.get() { "Abriendo..." } else { "Abrir archivo" }}
                    </button>
                </form>

                <Show when=move || !open_error.get().is_empty()>
                    <p class="home-error">{move || open_error.get()}</p>
                </Show>
            </section>
        </main>
    }
}
