# ADR 0006: Persisted recent files

## Status

Accepted

## Decision

`LogRegistry` owns persisted recent-file history through an internal
`RecentFilesManager` created from `ConfigStore`.

The history preserves stable route IDs for the ten most recently opened native
desktop or SSR server-root files. An unavailable or unauthorized persisted
route resolves as not found; the web client returns to the home route.

## Context

`LogRegistry` currently maps generated UUIDs to readers in memory. Restarting
the process loses that mapping, so a previously valid `/log/<id>` route cannot
be reopened.

ADR 0004 assigns configuration document naming and persistence infrastructure
to core through `ConfigStore`. ADR 0005 makes `LogRegistry` the final file-open
authorization boundary through `FileOpenPolicy`.

Recent-file restoration must use both decisions: core owns the document and
registry owns the authorization check before recreating a reader.

## Decisions

| Area | Decision |
|---|---|
| Ownership | `LogRegistry` owns one `RecentFilesManager`, alongside its internal visual-rules manager. |
| Configuration | `ConfigStore` owns the private `recent-files.json` document name. Web SSR and desktop supply only their configuration directory. |
| Stored record | Each entry contains a stable UUID, a canonical absolute path, and its last-opened timestamp. |
| Capacity | History retains at most 10 entries, ordered by most recent opening. |
| Deduplication | The canonical full path is the identity. Reopening it reuses its UUID and moves it to the front. Basenames are never matched. |
| Cache and durability | History loads once at registry construction and is kept in memory. Each mutation is merged and atomically persisted under a file lock. |
| Normal opening | Persistent opening validates and canonicalizes the path, reuses an existing history UUID when present, otherwise creates a new UUID and records it after the reader opens. |
| Restoration | `get_reader(id)` first checks open readers. If absent, it resolves the ID in history and recreates the reader with that same UUID. |
| Authorization | Every recreated reader passes through `FileOpenPolicy`; SSR restoration therefore cannot bypass the active `LOGMANCER_SERVER_FILE_ROOT`. |
| Uploads | Browser uploads are ephemeral: they create an in-memory reader but never enter history. |
| Missing records | Missing, unreadable, moved, or unauthorized paths result in not found. `LogView` redirects to `/` only for that response. |
| Startup files | Desktop positional command-line files are persistent openings. The future TUI follows the same rule. `LOGMANCER_INITIAL_FILE` and SSR initial-file startup are removed. |
| User interface | This change adds no recent-files UI. |

## Persistence model

The document is readable JSON with a versioned envelope:

```json
{
  "schemaVersion": 1,
  "entries": [
    {
      "id": "0cc4fad3-e0a0-440a-a857-5b809c1af57a",
      "path": "/var/log/application.log",
      "openedAt": "2026-08-26T12:00:00Z"
    }
  ]
}
```

The document records paths, not file content or file identity. Replacing a
file at the same path opens its current contents. Moving a configuration
directory to another machine can leave entries invalid; those entries are
treated as not found.

## Consequences

- Direct log routes survive application restart when their file remains valid.
- Desktop and SSR histories remain separate because their configuration
  directories are separate.
- TUI can use the same registry behavior without duplicating persistence or
  restoration logic.
- The configuration file contains filesystem paths and must remain in the
  private, ignored configuration directory.
- Concurrent processes do not silently overwrite each other's history updates.

## Out of scope

- A UI for viewing, selecting, or clearing recent files.
- Persistent browser uploads or upload cleanup.
- Detecting that a file at a stable path was replaced.
- Cross-machine path portability.

## Verification checklist

- [ ] Opening a new persistent file creates a record with a canonical path and UUID.
- [ ] Reopening the same canonical path reuses its UUID and updates its recency.
- [ ] The eleventh entry removes the least recently opened entry.
- [ ] Restart restoration recreates a reader with the persisted UUID.
- [ ] SSR restoration rejects paths outside the configured server root.
- [ ] Uploads never appear in `recent-files.json`.
- [ ] A missing or rejected route redirects the web client to `/`.
- [ ] Desktop positional startup records and opens its file; SSR does not use `LOGMANCER_INITIAL_FILE`.
