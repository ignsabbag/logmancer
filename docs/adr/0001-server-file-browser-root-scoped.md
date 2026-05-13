# ADR 0001: Root-scoped server file browser

## Status

Accepted

## Decision

Server-side browsing is an explicit, root-scoped capability enabled by `LOGMANCER_SERVER_FILE_ROOT`.

The browser lists and opens files only under that configured root. The UI uses a Spotlight-style modal, but filtering stays local to the currently loaded directory.

## Context

Logmancer Web needs a safer way to open server-side log files without requiring users to know exact paths. At the same time, exposing server files from a web process is sensitive: the feature must avoid arbitrary filesystem access and must not leak internal server paths through the UI or errors.

## Decisions

| Area | Decision |
|---|---|
| Enablement | Require `LOGMANCER_SERVER_FILE_ROOT`; if it is missing/invalid, server browsing is unavailable. |
| Scope | Browse and open only paths that resolve under the configured root. |
| Path validation | Canonicalize the configured root and each requested path before access. |
| Symlinks | Reject symlinks that resolve outside the configured root. |
| Search | Filter only the current directory entries; do not add recursive/global search. |
| File opening | Re-validate the selected file and confirm it is text-readable before opening it. |
| Errors | Return safe messages that avoid leaking absolute server paths or implementation details. |

## Consequences

- Deployments must opt in by setting `LOGMANCER_SERVER_FILE_ROOT`.
- Users can navigate logs without typing full server paths.
- Backend validation remains the source of truth; frontend state is only a convenience.
- Recursive search can be considered later as a separate feature with its own security and performance review.

## Verification checklist

- [ ] Missing/invalid `LOGMANCER_SERVER_FILE_ROOT` keeps server browsing unavailable.
- [ ] Valid `LOGMANCER_SERVER_FILE_ROOT` enables browsing from that root.
- [ ] Traversal and absolute-path escape attempts are rejected.
- [ ] Symlinks cannot escape the configured root.
- [ ] Filtering does not search outside the current directory listing.
- [ ] Opening a file revalidates scope and text-readability.
