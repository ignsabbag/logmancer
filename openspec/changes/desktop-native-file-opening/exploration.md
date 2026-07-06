## Exploration: desktop-native-file-opening drag/drop slice

### Current State
Slice 1 is implemented around a capability boundary. Web runtime exposes browser upload and server browser; desktop runtime hides both and exposes `desktop_native_open`. `Home` still has DOM `dragover`/`drop` handlers, but on desktop the current drop handler calls `open_native_file.run(())`, which opens the native picker rather than opening the dropped path. Web drag/drop continues to use browser `DataTransfer.files` and `upload_local_file`.

Desktop native opening already uses the safe path: `logmancer-web/src/file_opening.rs::open_native_log_file()` invokes Tauri command `open_native_log_file`; `logmancer-desktop/src/lib.rs` resolves the selected path and calls `open_selected_log_file`, which registers the path through the shared `Arc<LogRegistry>` used by `start_leptos_with_registry`.

Tauri v2 exposes local file drops as native paths. On the frontend, `getCurrentWebview().onDragDropEvent(...)` or `getCurrentWindow().onDragDropEvent(...)` receives payloads with `type: 'over' | 'drop' | 'cancel'`; `drop` includes `paths`. On the Rust side, `WindowEvent::DragDrop(DragDropEvent)` includes `Enter { paths, position }`, `Drop { paths, position }`, `Over { position }`, and `Leave`. Docs.rs shows these APIs for Linux, Windows, and macOS targets; Windows also has a low-level `drag_and_drop(bool)` window builder knob, but default Tauri webview file-drop support should be verified in packaged binaries.

OpenSpec note: `openspec/config.yaml` and main `openspec/specs/` are not present in this working tree, so this exploration follows the standard change-folder convention only.

### Affected Areas
- `logmancer-web/src/components/home.rs` — should stop treating desktop DOM drop as “open picker” and instead use a desktop-native drop path listener while leaving web browser drop/upload unchanged.
- `logmancer-web/src/file_opening.rs` — likely home for a narrow helper that invokes a new Tauri command with an explicit path, reusing the native-open boundary instead of upload.
- `logmancer-desktop/src/lib.rs` — should add a command such as `open_native_log_path(path: String)` that delegates to `open_selected_log_file` / `LogRegistry`, plus diagnostic logging for dropped paths.
- `logmancer-desktop/capabilities/default.json` — already has `core:default` and dialog permission; likely no extra permission is needed for drag/drop events themselves, but a new custom command permission may be generated/required by Tauri and must be checked after build/schema generation.
- `openspec/changes/desktop-native-file-opening/tasks.md` — already identifies desktop drag/drop as PR/slice 2; implementation tasks may need a small follow-up update during planning/apply.

### Approaches
1. **Frontend Tauri drag/drop listener + path command** — In desktop/hydrate runtime, attach `onDragDropEvent`, take the first dropped path, and invoke a command that opens that path through the shared registry.
   - Pros: Directly uses documented Tauri v2 webview API; keeps UI feedback in `Home`; avoids restoring browser upload on desktop; reuses existing registry open flow with a small command wrapper.
   - Cons: Requires JS interop from Rust/WASM for an API that is normally consumed from TypeScript; lifecycle cleanup/unlisten must be handled carefully.
   - Effort: Medium

2. **Rust window event handler opens dropped paths** — Handle `WindowEvent::DragDrop(DragDropEvent::Drop { paths, .. })` in desktop Rust and navigate/emit result to the webview.
   - Pros: Strong typed access to `PathBuf`; no frontend JS wrapper for drag/drop payload parsing.
   - Cons: More coupling between desktop shell and web navigation/error display; harder to reuse Home loading/error state; needs a frontend notification path for success/failure.
   - Effort: Medium-High

3. **Keep current desktop drop behavior as picker open** — Treat desktop drop as another way to open the native picker.
   - Pros: Almost no code change.
   - Cons: Violates user intent: dropped file path is ignored; diagnostic builds would not prove real native path drop.
   - Effort: Low

### Recommendation
Use Approach 1. Add a desktop-only Tauri drag/drop listener from the existing Home/file-opening boundary, and route the dropped path into a new Tauri command that delegates to the same `open_selected_log_file` / shared `LogRegistry` path used by picker open. Keep web DOM drag/drop unchanged: normal web runtime continues to read `DataTransfer.files` and upload through `/api/upload-file`.

Minimal implementation plan:
1. Add `open_native_log_path(path: String) -> Result<String, String>` in `logmancer-desktop/src/lib.rs`, implemented via `open_selected_log_file(state.registry.as_ref(), Some(PathBuf::from(path)))` and returning the `file_id`.
2. Add a hydrate-only helper in `logmancer-web/src/file_opening.rs` to listen for Tauri drag/drop events and invoke the new command on `drop` payload paths; keep it inert outside desktop runtime.
3. Wire Home desktop drop state to that helper; for web runtime, preserve existing `DataTransfer.files` upload behavior.
4. Log diagnostics at each boundary: event type, path count, selected first path basename/path debug as acceptable for diagnostic builds, command invocation, registry success/failure, and navigation target file_id.
5. Keep implementation within the review budget by avoiding UI redesign and not touching upload/server-browser behavior.

### Risks
- Tauri path payload availability must be proven with GitHub-built Linux and Windows binaries, not only local dev, because drag/drop behavior can vary by webview/packaging/desktop environment.
- The remote local HTTP origin (`http://127.0.0.1:*`) must still be allowed to use Tauri APIs; direct picker already exercises IPC, but drag/drop event delivery should be verified separately.
- Capability generation may require a custom command permission for `open_native_log_path`; drag/drop event listening itself appears to be core API rather than plugin-scoped permission, but this must be confirmed by build output and generated schema.
- Multiple-file drops need an explicit first-file-only decision or rejection; directories should fail cleanly through `LogRegistry::open_file` with a user-visible error.
- Logging full local paths is useful for diagnostic binaries but can leak sensitive paths; use targeted diagnostics and avoid broad permanent noisy logging.

### Ready for Proposal
Yes. This is ready for slice-2 apply planning: implement desktop native path drag/drop through Tauri v2 `onDragDropEvent` or equivalent Rust `WindowEvent::DragDrop`, reuse `LogRegistry`, preserve web upload drag/drop, and require GitHub-built Linux/Windows diagnostics before considering the slice fully proven.
