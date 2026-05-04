use crate::api::commons::{OpenServerFileRequest, OpenServerFileResponse};
use leptos::logging::log;
use leptos::prelude::*;
use leptos::*;
use leptos_router::hooks::use_navigate;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn Home() -> impl IntoView {
    let (server_path, set_server_path) = signal(String::new());
    let (open_error, set_open_error) = signal(String::new());
    let navigate = use_navigate();

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let path_value = server_path.get();
        let trimmed_path = path_value.trim().to_string();
        if trimmed_path.is_empty() {
            set_open_error.set("Ingresá una ruta válida.".to_string());
            return;
        }

        set_open_error.set(String::new());
        let navigate = navigate.clone();
        spawn_local(async move {
            let base = window().location().origin().unwrap();
            let url = format!("{}/api/open-server-file", base);
            match reqwest::Client::new()
                .post(url)
                .json(&OpenServerFileRequest { path: trimmed_path })
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<OpenServerFileResponse>().await {
                            Ok(response) => {
                                navigate(&format!("/log/{}", response.file_id), Default::default());
                            }
                            Err(err) => {
                                log!("Error parseando respuesta: {:?}", err);
                                set_open_error.set(
                                    "No se pudo interpretar la respuesta del servidor.".to_string(),
                                );
                            }
                        }
                    } else {
                        match response.json::<String>().await {
                            Ok(message) => {
                                log!("Error al abrir archivo: {}", message);
                                set_open_error.set(message);
                            }
                            Err(err) => {
                                log!("Error leyendo mensaje de error JSON: {:?}", err);
                                set_open_error
                                    .set("No se pudo abrir el archivo solicitado.".to_string());
                            }
                        }
                    }
                }
                Err(err) => {
                    log!("Error abriendo archivo: {:?}", err);
                    set_open_error.set("No se pudo conectar con el servidor.".to_string());
                }
            }
        });
    };
    view! {
        // <div style="border: 2px dashed gray; padding: 1rem; margin-bottom: 1rem;">
        //     <p>"📤 Drag your file here or use the button to select."</p>
        //     <form action="/api/upload_file" method="post" enctype="multipart/form-data">
        //         <input type="file" name="file" />
        //         <button type="submit">Subir</button>
        //     </form>
        // </div>
        <form class="home" on:submit=on_submit>
            <input
              type="text"
              placeholder="Ruta en el servidor"
              bind:value=(server_path, set_server_path)
            />
            <button type="submit">"Abrir archivo"</button>
        </form>
        <Show when=move || !open_error.get().is_empty()>
            <p class="home-error">{move || open_error.get()}</p>
        </Show>
    }
}
