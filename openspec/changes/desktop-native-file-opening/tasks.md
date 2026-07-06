# Tasks: Desktop Native File Opening

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 350-520 |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1: direct desktop native open + hide desktop upload; PR 2: optional desktop drag/drop after path safety check |
| Delivery strategy | ask-always |
| Chain strategy | pending |

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Desktop native open, capability-driven Home, web behavior preserved | PR 1 | Includes code, tests, capability permissions, changelog |
| 2 | Desktop local drag/drop only if Tauri path drop is proven safe | PR 2 | Deferred; must reuse native boundary and avoid upload UI |

## Phase 1: Capability Boundary

- [x] 1.1 Create `logmancer-web/src/file_opening.rs` with runtime capabilities for browser upload, root-scoped server browser, and desktop native open.
- [x] 1.2 Expose `file_opening` from `logmancer-web/src/lib.rs` for hydrate/browser builds.
- [x] 1.3 Keep `logmancer-web/src/api/config.rs` from registering `open_server_file`; preserve existing upload and server-browser routes.

## Phase 2: First Slice Implementation

- [x] 2.1 Update `logmancer-web/src/components/home.rs` to keep one Home screen and render actions from file-opening capabilities.
- [x] 2.2 Preserve web upload and drag/drop upload through existing `logmancer-web/src/browser_api_client.rs` upload flow.
- [x] 2.3 Hide browser upload/drop on desktop and show a desktop-native open-file action that navigates to `/log/{file_id}` on success.
- [x] 2.4 Update `logmancer-desktop/src/lib.rs` so Tauri manages the same `Arc<LogRegistry>` passed to `start_leptos_with_registry`.
- [x] 2.5 Add `open_native_log_file` in `logmancer-desktop/src/lib.rs` using native dialog selection and `LogRegistry::open_file`, returning `Option<String>`.
- [x] 2.6 Update `logmancer-desktop/Cargo.toml` and `logmancer-desktop/capabilities/default.json` for dialog plugin and command permissions.

## Phase 3: Verification

- [x] 3.1 Add/adjust Rust tests for capability mapping: web shows upload; desktop hides upload and exposes native open.
- [ ] 3.2 Preserve or extend `logmancer-web/src/api/server_browser.rs` tests for root scoping, traversal rejection, and no root disabled state.
- [x] 3.3 Verify normal web deployments cannot open arbitrary absolute paths through HTTP; `open_server_file` remains unregistered.
- [x] 3.4 Run focused web and desktop checks/tests for registry consistency.
- [ ] 3.5 Run `cargo clippy --workspace -- -D warnings`; smoke `cargo tauri dev` or document environment blocker.

## Phase 4: Release Notes And Deferred Work

- [x] 4.1 Update `CHANGELOG.md` under `[Unreleased]` because desktop native opening is user-facing.
- [x] 4.2 Leave desktop drag/drop as deferred unless Tauri path-drop behavior is proven safe in the same apply window.

## Phase 5: Desktop Native Drag/Drop Slice

- [x] 5.1 Add a desktop-only native drop path command that reuses the shared `LogRegistry::open_file` path used by the native picker.
- [x] 5.2 Listen for Tauri webview drag/drop path events from Home/file-opening boundary without restoring browser upload UI on desktop.
- [x] 5.3 Navigate the existing desktop window to `/log/{file_id}` after a successful dropped-path open.
- [x] 5.4 Preserve web drag/drop upload and the desktop picker button behavior.
- [x] 5.5 Add focused diagnostics that log event type/count/status without permanently logging full local paths.

## Review Follow-up Risks

- Direct `Home` DOM drag/drop behavior is not unit-tested because Leptos DOM drag/drop requires a browser/webview test harness. The browser upload path remains unchanged, and focused unit tests cover the runtime capability mapping plus first-path desktop drop selection outside the DOM.
- The native dropped-path IPC command is covered through a helper that reuses `LogRegistry::open_file` and rejects empty paths; the Tauri webview permission boundary is verified as static capability configuration without adding an arbitrary-path HTTP endpoint.
