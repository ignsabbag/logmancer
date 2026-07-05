# Proposal: Desktop Native File Opening

## Intent

Give desktop users native local-file opening without `LOGMANCER_SERVER_FILE_ROOT`, while preserving web upload and root-scoped server-browser security.

## Scope

### In Scope
- Keep Web Home upload, drag/drop upload, and current server browser.
- Hide upload from Desktop Home.
- Add a runtime file-opening capability/strategy boundary.
- Make desktop direct native open slice 1.
- Defer desktop local drag/drop to slice 2 if path-drop behavior is safe.
- Preserve root scoping; expose no arbitrary-path web endpoint.

### Out of Scope
- Replacing `/api/upload-file`.
- Broadening or auto-selecting unsafe server roots.
- General-purpose local-path web open endpoint.

## Capabilities

### New Capabilities
- `file-opening-capabilities`: Advertises and executes available strategies: browser upload, root-scoped server browser, and desktop native open.

### Modified Capabilities
- None

## Approach

Home consumes one runtime capability/strategy boundary instead of scattered desktop checks. Web exposes upload plus existing server-browser availability. Desktop exposes native open via Tauri dialog/IPC backed by the shared `LogRegistry`; upload is absent. Drag/drop can later reuse the native strategy only if Tauri/webview events provide safe local paths.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `logmancer-web/src/components/home.rs` | Modified | Render capability-driven actions. |
| `logmancer-web/src/browser_api_client.rs` | Modified | Consume capability status if needed. |
| `logmancer-web/src/api/config.rs` | Modified | Wire safe capability/status route only. |
| `logmancer-web/src/api/server_browser.rs` | Guarded | Preserve root-scoped browsing. |
| `logmancer-web/src/api/open_server_file.rs` | Guarded | Must not become general web route. |
| `logmancer-desktop/src/lib.rs` | Modified | Add native open and shared registry access. |
| `logmancer-desktop/Cargo.toml` | Modified | Add dialog dependency if selected. |
| `logmancer-desktop/capabilities/default.json` | Modified | Permit native dialog/command access. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Arbitrary path access leaks into web | Med | Keep native open behind Tauri IPC/capabilities. |
| Desktop command uses separate registry | Med | Share desktop `LogRegistry`. |
| Local HTTP origin lacks IPC permission | Med | Verify Tauri capability config. |
| Drag/drop lacks safe paths | Med | Defer until proven. |

## Rollback Plan

Revert the capability boundary and desktop native-open wiring. Web upload and server-browser routes remain unchanged.

## Dependencies

- Tauri native dialog/IPC permissions.
- Shared desktop/server `LogRegistry`.

## Success Criteria

- [ ] Web Home still supports upload, drag/drop upload, and root-scoped server browsing.
- [ ] Desktop Home shows native open and no browser upload UI.
- [ ] Desktop can open a local file without `LOGMANCER_SERVER_FILE_ROOT`.
- [ ] No normal web route accepts arbitrary filesystem paths.
