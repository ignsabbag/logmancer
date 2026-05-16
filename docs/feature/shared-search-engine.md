# Shared Search Engine

## Quick answer

Logmancer search is a **core-owned capability** shared by Web, Desktop, and TUI:

- Search behavior lives in `logmancer-core`, not in each client.
- `PageResult` remains the primary page response and may include optional search metadata.
- Clients render page-scoped match spans and highlight the current match.
- Navigation uses shared `next` / `previous` semantics with less-style wrapping.
- Search matches support multiple occurrences per line.

## User behavior

| Action | Expected result |
|---|---|
| Start a search | Core records the active query and exposes search status/metadata. |
| Read or scroll a page | The page response includes visible match spans when search is active. |
| Press next | Core moves to the next match and returns a page positioned around it. |
| Press previous | Core moves to the previous match and returns a page positioned around it. |
| Reach last/first match | Navigation wraps, matching less-style search behavior. |
| Clear search | Core removes active search metadata and normal page reads continue. |

## Response model

`PageResult` is still the source of truth for visible log content. Search metadata is composed into it only when a search is active.

```rust
PageResult {
    page: Vec<LogLine>,
    search: Option<PageSearchResult>,
    // existing page metadata...
}
```

`PageSearchResult` carries only what clients need to render and navigate the current view:

- active query,
- search status,
- total known matches,
- current match index,
- current match identity,
- match spans for the visible page.

## Match identity

A match is not just a line number. A single log line may contain multiple matches, so each match is identified by:

| Field | Purpose |
|---|---|
| `line_index` | Global log line containing the match. |
| `start` / `end` | Intra-line span to highlight. |
| `ordinal` | Global match position used for navigation. |
| `is_current` | Whether this visible match is the selected match. |

## Rendering contract

Clients are thin adapters:

- Web/Desktop render all `page_matches` spans and emphasize the current one.
- TUI renders visible matches and marks the current match.
- Clients do not compute global search state or decide navigation order.

This keeps UI rendering flexible while preventing each client from inventing different search semantics.

## Responsiveness requirement

Search must remain responsive on large log files.

The target architecture is asynchronous search indexing:

- starting a search should return quickly,
- a search worker indexes matches in batches,
- write locks should be held only while merging batch results,
- stale search batches should not update state after a newer query starts,
- `total_matches` may be partial while indexing is still in progress.

This mirrors the existing worker-based indexing/filtering model and avoids blocking UI/API calls while scanning large files.

## Review checklist

- [ ] Search behavior is owned by `logmancer-core`.
- [ ] `PageResult` remains the primary response contract.
- [ ] Search metadata is optional and page-scoped.
- [ ] Multiple matches per line are represented with line + span + ordinal.
- [ ] `next` / `previous` navigation wraps.
- [ ] Scroll position does not change the selected current match.
- [ ] Web/Desktop and TUI only render core-provided search metadata.
- [ ] Large-file search work is batchable and does not require one long synchronous scan.
