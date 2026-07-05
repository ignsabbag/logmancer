## Exploration: desktop-native-file-opening

### Current State
`logmancer-web/src/components/home.rs` is the shared Home screen for browser and desktop. It always renders browser-style local file upload, drag/drop upload, a server-browser divider, and the `ServerFileSpotlight` entry point. Upload posts a browser `File` to `/api/upload-file`; drag/drop also uses that upload path.

Server-side browsing is intentionally root-scoped. `logmancer-web/src/api/server_browser.rs` enables browsing only when `LOGMANCER_SERVER_FILE_ROOT` resolves to a valid directory, validates relative path tokens against that root, rejects escapes, and checks selected files are text-readable before opening them through `LogRegistry`.

`logmancer-desktop/src/lib.rs` starts a Tauri shell, creates a shared `LogRegistry`, starts the embedded Leptos SSR server, and navigates the main window to that local server. It reuses the same web Home route, so desktop currently shows upload UI and still depends on `LOGMANCER_SERVER_FILE_ROOT` for browsing. The desktop crate has `tauri-plugin-opener`, but no native dialog plugin or file-opening command.

There is an unused `logmancer-web/src/api/open_server_file.rs` handler and `OpenServerFileRequest` DTO that can open an arbitrary path through `LogRegistry`, but the route is not registered in `api_routes_with_registry`. Exposing that handler broadly would conflict with the accepted root-scoped server-browser ADR unless it is constrained to a desktop-only/native capability boundary.

OpenSpec note: `openspec/config.yaml` and main `openspec/specs/` are not present in this working tree, so this exploration follows the standard change-folder convention only.

### Affected Areas
- `logmancer-web/src/components/home.rs` — current shared Home UX mixes browser upload, desktop needs, and server-browser status in one component.
- `logmancer-web/src/browser_api_client.rs` — existing browser HTTP client owns upload and server-browser calls; likely place for a small runtime-capability/status client if served over HTTP.
- `logmancer-web/src/api/config.rs` — route/state boundary for any server-side capability endpoint or desktop-only open route.
- `logmancer-web/src/api/server_browser.rs` — current server browser must remain root-scoped for web deployments.
- `logmancer-web/src/api/open_server_file.rs` — currently unused direct path opener; tempting reuse point but unsafe as a general web route.
- `logmancer-desktop/src/lib.rs` — Tauri setup would add native dialog/open integration, plugin initialization, commands, or capability injection.
- `logmancer-desktop/Cargo.toml` — would need `tauri-plugin-dialog` for native file picking if using the plugin.
- `logmancer-desktop/capabilities/default.json` — would need dialog permissions and possibly custom command permissions.
- `docs/adr/0001-server-file-browser-root-scoped.md` — frames the security constraint for server browsing and should not be weakened by web routes.

### Approaches
1. **Register direct `/api/open-server-file` for desktop and hide upload with a runtime mode flag** — Add a server route that opens absolute paths and expose it only when the embedded desktop server is running.
   - Pros: Reuses `LogRegistry` and existing HTTP/navigation flow; minimal frontend invocation surface; direct native dialog can return a path and the browser client can post it.
   - Cons: Easy to accidentally expose arbitrary filesystem access in web/server mode; needs a reliable desktop-only state/config gate; still couples desktop native selection to web API routes.
   - Effort: Medium

2. **Tauri command owns native dialog and registry open, Home consumes runtime file-opening capabilities** — Desktop registers a command such as `open_native_file`, backed by Tauri dialog selection and the same desktop `LogRegistry`; web Home renders available actions from a capability/strategy boundary instead of hard-coded platform checks.
   - Pros: Keeps native file access inside Tauri capability permissions; avoids enabling arbitrary path opening in normal web deployments; cleanly separates browser upload, server browser, and desktop native open strategies; aligns with the request to avoid scattered `if desktop` checks.
   - Cons: Requires wiring the web WASM side to Tauri IPC and preserving SSR/browser builds; the `LogRegistry` is currently owned by the embedded server task, so desktop command and server must share it deliberately.
   - Effort: Medium-High

3. **Make desktop set `LOGMANCER_SERVER_FILE_ROOT` automatically and keep current server browser** — On startup, desktop configures the server browser root to a broad user directory and hides upload.
   - Pros: Small conceptual change; reuses the existing Spotlight browser and root-bound validation.
   - Cons: Does not provide a native file dialog; still requires a configured root, just implicitly; broad roots weaken the safety and UX intent; does not solve direct local opening well.
   - Effort: Low-Medium

4. **Use upload path for desktop drag/drop/direct selection** — Keep browser `File` upload for desktop local files and only alter labels/visibility.
   - Pros: Very small UI change; drag/drop already works as a browser file drop.
   - Cons: Still copies the file through multipart upload, does not feel native, keeps unnecessary browser upload machinery in desktop, and does not support direct OS file opening without upload.
   - Effort: Low

### Recommendation
Use Approach 2 as the design target, with runtime capabilities as the boundary: Home should ask what file-opening strategies are available and render browser upload, server browser, or desktop native open based on those capabilities. The web/browser deployment keeps upload and current root-scoped server browser behavior. The desktop deployment hides upload and exposes a native `Open File` action backed by Tauri dialog permissions and the shared desktop `LogRegistry`.

Treat local file drag/drop as a second slice. The first slice should establish the strategy boundary and direct native open path; the second can map desktop drag/drop into the same native-open strategy if Tauri/webview drag-drop APIs provide stable paths without falling back to multipart upload.

Do not make `/api/open-server-file` generally available for web. If it is reused, gate it behind explicit desktop runtime state and keep it unavailable in normal `logmancer-web` server mode; otherwise prefer a Tauri command so arbitrary path access remains within desktop IPC/capability permissions.

### Risks
- Tauri IPC from a Leptos app served from `http://127.0.0.1:{port}` may need capability configuration attention because the desktop window navigates to a local HTTP URL rather than bundled static assets.
- Sharing `LogRegistry` between the embedded server and Tauri commands must avoid creating a second registry; otherwise the command could open a file ID that the web route cannot read.
- Exposing arbitrary path opening as an HTTP endpoint would be a security regression for web/server deployments if the gate is wrong.
- Desktop drag/drop may surface browser `File` objects rather than filesystem paths; path-based native drop should be validated against Tauri v2 capabilities before committing to that slice.
- Hiding upload in desktop should not remove browser upload behavior from web Home or break existing `/api/upload-file` support.

### Ready for Proposal
Yes. The proposal should define a two-slice change: first introduce a runtime file-opening capability/strategy boundary and desktop native direct open; then add desktop local drag/drop if the Tauri path-drop behavior is confirmed. It should explicitly preserve web upload, preserve root-scoped server browsing, and avoid broad platform checks scattered through Home.
