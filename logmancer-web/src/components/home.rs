use crate::browser_api_client::{fetch_server_browser_status, upload_local_file};
use crate::components::ServerFileSpotlight;
use crate::file_opening::{
    detect_file_opening_runtime, initial_file_opening_capabilities, open_native_log_file,
    resolve_file_opening_capabilities,
};
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
    let initial_capabilities = initial_file_opening_capabilities();
    log!(
        "Home initial file opening capabilities: {:?}",
        initial_capabilities
    );
    let (file_opening_capabilities, set_file_opening_capabilities) = signal(initial_capabilities);
    let (is_server_browser_enabled, set_is_server_browser_enabled) = signal(false);
    let (server_browser_message, set_server_browser_message) =
        signal("Checking server browser availability...".to_string());
    let (is_loading_server_browser_status, set_is_loading_server_browser_status) = signal(true);
    let navigate = use_navigate();
    let navigate_for_upload = navigate.clone();

    Effect::new(move |_| {
        let runtime = detect_file_opening_runtime();
        let capabilities = resolve_file_opening_capabilities(runtime);
        log!(
            "Home updated file opening capabilities: runtime={:?} capabilities={:?}",
            runtime,
            capabilities
        );
        set_file_opening_capabilities.set(capabilities);
    });

    Effect::new(move |_| {
        let capabilities = file_opening_capabilities.get();
        if !capabilities.server_browser {
            log!(
                "Skipping server browser status fetch because capability is hidden: {:?}",
                capabilities
            );
            set_is_server_browser_enabled.set(false);
            set_is_loading_server_browser_status.set(false);
            return;
        }

        log!(
            "Fetching server browser status because capability is visible: {:?}",
            capabilities
        );
        set_is_loading_server_browser_status.set(true);
        spawn_local(async move {
            match fetch_server_browser_status().await {
                Ok(status) => {
                    log!(
                        "Server browser status fetched: enabled={} message={:?}",
                        status.enabled,
                        status.message
                    );
                    set_is_server_browser_enabled.set(status.enabled);
                    set_server_browser_message.set(status.message.unwrap_or_else(|| {
                        "Browse and open files inside the configured server root.".to_string()
                    }));
                }
                Err(error) => {
                    log!("Server browser status fetch failed: {}", error);
                    set_is_server_browser_enabled.set(false);
                    set_server_browser_message.set(error);
                }
            }
            set_is_loading_server_browser_status.set(false);
        });
    });

    let navigate_for_native_open = navigate.clone();
    let open_native_file = Callback::new(move |()| {
        log!("Home native open requested");
        set_upload_error.set(String::new());
        set_is_uploading.set(true);
        let navigate = navigate_for_native_open.clone();

        spawn_local(async move {
            match open_native_log_file().await {
                Ok(Some(file_id)) => {
                    log!("Home native open succeeded file_id={}", file_id);
                    navigate(&format!("/log/{file_id}"), Default::default());
                }
                Ok(None) => {
                    log!("Home native open cancelled");
                }
                Err(error) => {
                    log!("Error opening native file: {}", error);
                    set_upload_error.set(error);
                }
            }

            set_is_uploading.set(false);
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

        if file_opening_capabilities
            .get_untracked()
            .desktop_native_open
        {
            open_native_file.run(());
            return;
        }

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
                <p class="home-subtitle">"Explore large logs from your browser without the friction."</p>

                <Show when=move || {
                    let capabilities = file_opening_capabilities.get();
                    capabilities.browser_upload || capabilities.desktop_native_open
                }>
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
                        <p class="home-dropzone-title">
                            {move || {
                                if file_opening_capabilities.get().desktop_native_open {
                                    "Open a local log file"
                                } else {
                                    "Drag and drop a local file"
                                }
                            }}
                        </p>
                        <p class="home-dropzone-subtitle">
                            {move || {
                                if file_opening_capabilities.get().desktop_native_open {
                                    "Choose a file from this computer with the native desktop picker"
                                } else {
                                    "or choose one manually to upload it"
                                }
                            }}
                        </p>

                        <Show when=move || file_opening_capabilities.get().browser_upload>
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
                                {move || if is_uploading.get() { "Uploading..." } else { "Choose local file" }}
                            </label>
                        </Show>

                        <Show when=move || file_opening_capabilities.get().desktop_native_open>
                            <button
                                type="button"
                                class=move || {
                                    if is_uploading.get() {
                                        "home-upload-btn is-disabled"
                                    } else {
                                        "home-upload-btn"
                                    }
                                }
                                disabled=move || is_uploading.get()
                                on:click=move |_| open_native_file.run(())
                            >
                                {move || if is_uploading.get() { "Opening..." } else { "Choose local file" }}
                            </button>
                        </Show>
                    </div>
                </Show>

                <Show when=move || !upload_error.get().is_empty()>
                    <p class="home-error">{move || upload_error.get()}</p>
                </Show>

                <Show when=move || file_opening_capabilities.get().server_browser>
                    <div class="home-divider">
                        <span>"or open from the server"</span>
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
                                    "Checking..."
                                } else {
                                    "Explore Server"
                                }
                            }}
                        </button>
                        <p class="home-server-help">{move || server_browser_message.get()}</p>
                    </div>
                </Show>
            </section>

            <ServerFileSpotlight is_open=is_spotlight_open set_is_open=set_is_spotlight_open />
        </main>
    }
}
