use leptos::prelude::*;
use leptos::*;
use leptos::logging::log;
use wasm_bindgen_futures::spawn_local;
use leptos_router::hooks::use_navigate;
use crate::api::commons::{OpenServerFileRequest, OpenServerFileResponse};

#[component]
pub fn Home() -> impl IntoView {
    let (server_path, set_server_path) = signal(String::new());

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let path_value = server_path.get();
        spawn_local(async move {
            let base = window().location().origin().unwrap();
            let url = format!("{}/api/open-server-file", base);
            match reqwest::Client::new()
                .post(url)
                .json(&OpenServerFileRequest { path: path_value})
                .send()
                .await
            {
                Ok(response) => {
                    match response.json::<OpenServerFileResponse>().await {
                        Ok(response) => {
                            let navigate = use_navigate();
                            navigate(&format!("/log/{}", response.file_id), Default::default());
                        }
                        Err(err) => {
                            log!("Error parseando respuesta: {:?}", err);
                        }
                    }
                }
                Err(err) => {
                    log!("Error abriendo archivo: {:?}", err);
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
        <form on:submit=on_submit>
            <input
              type="text"
              placeholder="Ruta en el servidor"
              bind:value=(server_path, set_server_path)
            />
            <button type="submit">"Abrir archivo"</button>
        </form>
    }
}