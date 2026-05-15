# Server File Browser

## Quick answer

`Explore Server` is now **config-driven and root-scoped**:

- Set `LOGMANCER_SERVER_FILE_ROOT` to enable server browsing.
- If missing or invalid, Home keeps `Explore Server` disabled with a safe explanation.
- Manual server path entry is not part of the server browser experience.
- Spotlight filter is **local to the current directory only** (no recursive/global search).

## Configuration

### Enable server browsing

```bash
LOGMANCER_SERVER_FILE_ROOT=/path/to/logs
```

The path must resolve to a readable directory for the running Logmancer process.

### Disabled behavior

If `LOGMANCER_SERVER_FILE_ROOT` is missing, invalid, or unreadable:

- backend reports browser as unavailable,
- Home renders `Explore Server` disabled,
- UI shows a safe message explaining that server browsing requires configuration.

## Home behavior (source of truth)

| State | Home UI |
|---|---|
| Root configured + valid | `Explore Server` enabled |
| Root missing/invalid | `Explore Server` disabled with safe message |

Server browsing is exposed through `Explore Server` in both states.

## Spotlight behavior

- Starts at configured root.
- Shows current server path as read-only.
- Lists only current directory entries.
- Sorts folders first, then alphabetical.
- Supports enter folder, go up, single-select file, Enter/double-click/Open Selected.
- Filter applies only to currently loaded entries.

## Security and scope

- Read-only operations only.
- All list/open requests are validated against configured root.
- Traversal (`..`), absolute-path escape, and symlink-outside-root are rejected.
- Open re-validates path and checks text-readability before opening.
- Errors are safe and do not include internal absolute paths.

## Review checklist

- [ ] `LOGMANCER_SERVER_FILE_ROOT` valid → Explore Server enabled.
- [ ] `LOGMANCER_SERVER_FILE_ROOT` missing/invalid → Explore Server disabled + safe explanation.
- [ ] Server browsing is exposed through `Explore Server` only.
- [ ] Filtering does not search recursively.
- [ ] Open still routes to `/log/:id` only for valid text-readable files inside root.
