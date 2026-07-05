# Design: Desktop Native File Opening

## Technical Approach

Introduce one file-opening strategy boundary consumed by `Home`, then wire desktop native opening through Tauri IPC instead of an HTTP arbitrary-path route. Web keeps browser upload and root-scoped server browsing exactly as today; desktop advertises a native open action, hides browser upload, opens the chosen path into the shared `LogRegistry`, and navigates to `/log/{file_id}`.

## Architecture Decisions

| Decision | Choice | Alternatives considered | Rationale |
|---|---|---|---|
| Runtime boundary | Add a small `file_opening` client module used by `Home` to return available strategies and execute them. | Scatter `if desktop` checks in `home.rs`; backend-only capability route. | Keeps platform branching in one place. A client boundary can detect `window.__TAURI__` for desktop while still calling existing HTTP APIs for web server-browser status. |
| Desktop direct open | Add a Tauri command, e.g. `open_native_log_file`, that shows a native file dialog and calls `LogRegistry::open_file` on selection. | Register `/api/open-server-file`; auto-set `LOGMANCER_SERVER_FILE_ROOT`; keep upload-only desktop. | Tauri IPC/capabilities are the correct security boundary for arbitrary local path access. HTTP stays safe for normal web deployments. |
| Registry sharing | Store the existing desktop `Arc<LogRegistry>` in Tauri managed state and pass the same clone to `start_leptos_with_registry`. | Create a registry in the command; send path back to web and open over HTTP. | `/log/{file_id}` is served by the embedded Leptos server, so the command and server must write/read the same registry session. |
| Drag/drop | Defer desktop local drag/drop to a follow-up slice. | Implement native path-drop now; keep browser drop upload on desktop. | Direct open delivers the approved safe path first. Tauri/webview path-drop behavior needs separate verification and must reuse the native boundary. |
| Changelog | Update `CHANGELOG.md` under `[Unreleased] -> Added` during implementation. | No changelog entry. | This is user-facing desktop behavior and release-note-worthy. |

## Data Flow

Web upload:

    Home -> file_opening strategy -> /api/upload-file -> AppState.registry -> /log/{file_id}

Web server browser:

    Home -> ServerFileSpotlight -> /api/server-browser/* -> root-bound validation -> AppState.registry

Desktop native open:

    Home -> file_opening strategy -> window.__TAURI__.core.invoke("open_native_log_file")
         -> tauri-plugin-dialog file picker -> shared Arc<LogRegistry>.open_file(path)
         -> file_id -> navigate("/log/{file_id}") -> embedded Leptos server reads same registry

## File Changes

| File | Action | Description |
|---|---|---|
| `logmancer-web/src/file_opening.rs` | Create | Define `FileOpeningCapabilities` / strategy helpers, detect Tauri via `window.__TAURI__`, call upload, server-browser status, or native invoke. |
| `logmancer-web/src/lib.rs` | Modify | Expose the new web module to hydrate builds. |
| `logmancer-web/src/components/home.rs` | Modify | Render from capabilities: web upload/drop + server browser; desktop native button + server browser status; no desktop upload/drop. |
| `logmancer-web/src/browser_api_client.rs` | Modify | Keep existing upload/server-browser functions; optionally add a focused native invoke wrapper if not placed in `file_opening.rs`. |
| `logmancer-web/src/api/config.rs` | No arbitrary open route | Keep `/upload-file` and `/server-browser/*`; do not register `open_server_file`. |
| `logmancer-web/src/api/open_server_file.rs` | Leave unregistered or delete later | Must not become a general web endpoint. |
| `logmancer-desktop/src/lib.rs` | Modify | Add managed registry state, command handler, dialog plugin init, and shared registry cloning. |
| `logmancer-desktop/Cargo.toml` | Modify | Add `tauri-plugin-dialog = "2"` if using the plugin dialog API. |
| `logmancer-desktop/capabilities/default.json` | Modify | Permit dialog open and the custom command for `main`. |
| `CHANGELOG.md` | Modify | Add user-facing desktop native open entry under `[Unreleased]`. |

## Interfaces / Contracts

```rust
#[derive(Clone)]
struct DesktopState {
    registry: Arc<LogRegistry>,
}

#[tauri::command]
async fn open_native_log_file(state: tauri::State<'_, DesktopState>) -> Result<Option<String>, String>;
```

Frontend contract: native open returns `Ok(Some(file_id))` for a selected file, `Ok(None)` for cancel, and `Err(message)` for open/dialog failures. `Home` navigates only on `Some(file_id)`.

## Testing Strategy

| Layer | What to Test | Approach |
|---|---|---|
| Unit | Capability mapping and web upload visibility vs detected Tauri runtime. | Extract pure helpers and test with Rust `cargo test -p logmancer-web` where possible. |
| Unit | Server browser still rejects absolute paths, traversal, symlink escapes. | Preserve/extend existing `server_browser.rs` tests. |
| Integration | No `/api/open-server-file` route exists in normal web router. | Axum router test or route-level negative check if practical. |
| Desktop/manual | Dialog opens a local text log without `LOGMANCER_SERVER_FILE_ROOT` and navigates to `/log/{file_id}`. | `cargo tauri dev` plus release smoke. |
| Quality gate | Workspace compiles and lint passes. | `cargo test`, `cargo clippy --workspace -- -D warnings`; desktop build smoke if environment supports Tauri dependencies. |

## Migration / Rollout

No data migration required. Ship as one direct-open slice; drag/drop follows only after native path-drop safety is verified.

## Open Questions

- [ ] Confirm the exact Tauri v2 permission identifier generated for `open_native_log_file` and dialog open in `capabilities/default.json`.
