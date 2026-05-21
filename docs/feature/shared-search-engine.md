# Shared Search Engine

## Quick answer

Logmancer search is a **core-owned capability** shared by Web, Desktop, and TUI:

- Search behavior lives in `logmancer-core`, not in each client.
- `PageResult` remains the primary page response and may include optional search metadata.
- Web/Desktop expose search through a bottom search panel opened with `/` or `Ctrl+F`.
- Clients render page-scoped match spans and highlight the current match.
- Navigation uses shared `next` / `previous` semantics with less-style wrapping.
- Search matches support multiple occurrences per line.

## User behavior

| Action | Expected result |
|---|---|
| Start a search | Core records the active query and begins searching from the current position. |
| Read or scroll a page | The page response includes visible match spans when search is active. |
| Press next | Core moves to the next match and returns a page positioned around it. |
| Press previous | Core moves to the previous match and returns a page positioned around it. |
| Reach last/first match | Navigation wraps, matching less-style search behavior. |
| Clear search | Core removes active search metadata and normal page reads continue. |

## Web/Desktop search panel

Issue #21 adds the Web/Desktop entry point for shared search. It should stay focused on starting a search, revealing the first match, and rendering matches already visible on the current page.

| Interaction | Expected result |
|---|---|
| Press `/` | Open the bottom search panel and focus the input. |
| Press `Ctrl+F` | Open the same search panel and focus the input. |
| Type query | Update the input value without moving focus away from the search panel. |
| Press `Enter` | Submit the query, run shared search, reveal or select the first returned match, keep the panel visible, then move focus back to the main log panel. |
| Press `Esc` | Close the search panel cleanly without submitting a new search. |
| Click `x` | Close the search panel cleanly. |
| Submit an empty query | Reset search cleanly without stale highlights while keeping the panel visible. |

The panel is hidden by default. When visible, it is a compact fixed panel pinned to the bottom of the whole viewport/window. It spans the full window width, with the search input and `x` close button aligned on the right. The input receives focus immediately when opened or reopened with `/` or `Ctrl+F` so keyboard search works without an extra click. `Esc` and `x` are the only close interactions for issue #21; submitting with `Enter` leaves the panel visible.

Navigation between matches with `n` / `N` is intentionally out of scope for issue #21 and belongs to issue #23. Issue #21 may introduce state that makes #23 straightforward, but it should not implement next/previous navigation behavior.

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

The target architecture is asynchronous, position-aware search indexing:

- starting a search should return quickly,
- a search worker indexes matches in batches from the current position,
- after reaching the end of the indexed log, the worker wraps to the beginning and continues until it reaches the original start position,
- write locks should be held only while merging batch results,
- stale search batches should not update state after a newer query starts,
- `total_matches` may be partial while indexing is still in progress.

This mirrors the existing worker-based indexing/filtering model and avoids blocking UI/API calls while scanning large files.

## Search startup flow

Starting a search has two goals: return quickly and find the first useful result near the user.

```text
current page ── apply search ── enqueue worker from current line
      │                              │
      │                              ├─ scan current line → end
      │                              └─ wrap start → original line
      └─ bounded initial wait (<=500ms) then return PageResult
```

The client should not assume the first response contains a completed result set. Instead:

1. trigger search from the current page/line,
2. show a searching indicator while status is indexing,
3. poll search status or page data using the existing polling style,
4. jump to the first discovered match once core exposes it,
5. keep polling until the search status is ready.

## Partial results contract

While the search worker is indexing:

- visible page matches may be available before the global search is complete,
- `total_matches` is known-so-far, not final,
- the UI should avoid presenting final “N of M” semantics until status is ready,
- `current_match` may appear as soon as the first occurrence is discovered,
- a newer query invalidates older worker batches through a generation/id guard.

The worker may discover matches in circular scan order for responsiveness, but stable match identity remains based on global log position: line index, intra-line span, and ordinal.

## Review checklist

- [ ] Search behavior is owned by `logmancer-core`.
- [ ] `PageResult` remains the primary response contract.
- [ ] Search metadata is optional and page-scoped.
- [ ] Web/Desktop `/` opens a bottom search panel with focused input.
- [ ] Web/Desktop `Ctrl+F` opens the same search panel with focused input.
- [ ] Web/Desktop `Esc` and `x` close the search panel cleanly.
- [ ] Web/Desktop `Enter` submits search, keeps the panel visible, and returns focus to the main log panel.
- [ ] Multiple matches per line are represented with line + span + ordinal.
- [ ] `next` / `previous` navigation wraps in core/API; Web/Desktop `n` / `N` controls are intentionally deferred to issue #23.
- [ ] Scroll position does not change the selected current match.
- [ ] Web/Desktop and TUI only render core-provided search metadata.
- [ ] Large-file search work is batchable and does not require one long synchronous scan.
- [ ] Search indexing starts from the current position and wraps to the beginning after EOF.
- [ ] Clients show an indexing/searching state and poll until the first/current match is available.
- [ ] Partial totals are not presented as final totals while indexing is still running.
