use crate::components::async_functions::{fetch_server_browser_status, upload_local_file};
use crate::components::ServerFileSpotlight;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::*;
use leptos_router::hooks::use_navigate;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{DragEvent, Event, File, HtmlInputElement};

#[component]
pub fn Home() -> impl IntoView {
    let (upload_error, set_upload_error) = signal(String::new());
    let (is_uploading, set_is_uploading) = signal(false);
    let (is_dragging, set_is_dragging) = signal(false);
    let (is_spotlight_open, set_is_spotlight_open) = signal(false);
    let (is_server_browser_enabled, set_is_server_browser_enabled) = signal(false);
    let (server_browser_message, set_server_browser_message) =
        signal("Verificando disponibilidad del servidor...".to_string());
    let (is_loading_server_browser_status, set_is_loading_server_browser_status) = signal(true);
    let navigate = use_navigate();
    let navigate_for_upload = navigate.clone();

    Effect::new(move |_| {
        spawn_local(async move {
            match fetch_server_browser_status().await {
                Ok(status) => {
                    set_is_server_browser_enabled.set(status.enabled);
                    set_server_browser_message.set(status.message.unwrap_or_else(|| {
                        "Explorá y abrí archivos dentro del directorio configurado.".to_string()
                    }));
                }
                Err(error) => {
                    set_is_server_browser_enabled.set(false);
                    set_server_browser_message.set(error);
                }
            }
            set_is_loading_server_browser_status.set(false);
        });
    });

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

                <div class="home-server-form">
                    <button
                        type="button"
                        disabled=move || {
                            is_loading_server_browser_status.get() || !is_server_browser_enabled.get()
                        }
                        on:click=move |_| set_is_spotlight_open.set(true)
                    >
                        {move || {
                            if is_loading_server_browser_status.get() {
                                "Verificando..."
                            } else {
                                "Explorar servidor"
                            }
                        }}
                    </button>
                    <p class="home-server-help">{move || server_browser_message.get()}</p>
                </div>
            </section>

            <ServerFileSpotlight is_open=is_spotlight_open set_is_open=set_is_spotlight_open />
        </main>
    }
}
