# Logmancer Roadmap

This roadmap describes the current direction for Logmancer. It is intentionally high-level: concrete implementation details should live in GitHub issues, design notes, or specifications when needed.

## Current Version: 0.1.0

Logmancer currently provides the core experience for fast log viewing and navigation:

- Optimized file visualization and indexing.
- `less`-style navigation.
- Filter panel in web/desktop.
- Follow mode with `f` in TUI and web.
- `g` / `G` navigation.

Known gap:

- Web follow mode works, but does not yet have a clear visual indicator.

## Planned Releases

### 0.2.0 — Selection and Panel Synchronization

Improve the reading experience and make the relationship between the main panel and filter panel clearer.

- Highlight the selected line across viewer panels with a background color.
- Synchronize filter panel selection with the corresponding line in the main panel.
- Improve visual indication for the active/focused panel.
- Improve scroll behavior so navigation feels natural and predictable.
- Add a clear visual indicator for follow mode in web/desktop.

### 0.3.0 — Search

Add first-class search capabilities inspired by `less`.

- Search with `/`.
- Navigate matches with `n` / `N`.
- Highlight search matches.
- Show the current search query and match position when applicable.
- Implement search consistently across web/desktop/TUI where the feature applies.

### 0.4.0 — Visual Rules

Add configurable visual rules for highlighting important log lines without necessarily hiding other lines.

- Define rules based on text or regex matches.
- Define how visual metadata is represented in returned lines, such as a `LineStyle` or equivalent decoration model.
- Configure foreground and background colors.
- Provide a UI to create and edit visual rules.
- Persist visual rules configuration for reuse.
- Apply visual rules to matching lines.
- Support rule priority when multiple rules match the same line.
- Keep visual rules conceptually separate from filters:
  - filters decide which lines are visible;
  - visual rules decide how matching lines are displayed.

### 0.5.0 — Selected Text Occurrence Search

Add fast exploratory search based on selected text, inspired by tools like `glogg`.

- Select text and search for previous/next occurrences.
- Use shortcuts such as `*` and `#` where appropriate.
- Highlight all occurrences of the selected text.
- Reuse the same underlying search engine used by `/` search.

### 0.6.0 — Multi-file Workspace

Improve workflows that involve multiple open log files.

- Support multiple open files through the existing log registry model.
- Define the active file and how users switch between open files.
- Provide tabs, a file menu, or an equivalent navigation model for open files.
- Preserve relevant per-file state such as position, selection, search, filters, visual rules, and follow mode.
- Define the desktop strategy for Tauri, starting with a single-window tabs/menu model and leaving multi-window support as a future possibility.

### 0.7.0 — Structured Log Parsing

Move beyond plain text viewing by supporting structured log formats.

- Parse lines using known patterns such as Log4j, CSV, JSON Lines, or custom regex patterns.
- Display parsed logs in a table-like view.
- Support common columns such as timestamp, level, source/logger, thread, message, and custom fields.
- Allow filtering by parsed columns.

### 0.8.0 — Advanced TUI Experience

Bring the most relevant advanced capabilities to the terminal experience without overloading the interface.

- Ensure search features work well in the TUI.
- Support occurrence highlighting where terminal capabilities allow it.
- Consider structured/table views for parsed logs.
- Evaluate whether a dedicated filter panel makes sense for the TUI.

## Future Ideas

These ideas are not assigned to a specific release yet.

- Visual indicator for follow mode in web/desktop.
- Bookmarks for important lines.
- Export filtered or selected lines.
- Saved sessions with searches, filters, visual rules, and file state.
- Search and filter history.
- Log metrics and summaries, such as error/warning counts or distribution over time.
- Compare two log files.
- Automatic log format detection.
- Notes or annotations attached to lines.

## Planning Notes

- Features that are part of the core log navigation model should be designed core-first and exposed consistently across web/desktop/TUI when applicable.
- The UI does not need to be identical across platforms, but behavior should be predictable and shared where possible.
- Large features should be tracked as GitHub milestones and split into small, actionable issues with clear acceptance criteria.
