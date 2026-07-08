# Logmancer Roadmap

This roadmap describes the current direction for Logmancer. It is intentionally high-level: concrete implementation details should live in GitHub issues, design notes, or specifications when needed.

## Current Version: 0.3.0

Logmancer currently provides the core experience for fast log viewing, navigation, and search:

- Optimized file visualization and indexing.
- `less`-style navigation.
- Filter panel in web/desktop.
- Follow mode with `f` in TUI and web.
- `g` / `G` navigation.
- Selection and panel synchronization improvements.
- `less`-style search with `/`, `n`, and `N`.
- Search match highlighting and search status information where applicable.

Known gap:

- The roadmap should continue being translated into concrete GitHub milestones and issues before implementation work starts.

## Planned Releases

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
- Evaluate shortcuts for selected-text occurrence search.
- Consider structured/table views for parsed logs.
- Evaluate whether a dedicated filter panel makes sense for the TUI.

## Future Ideas

These ideas are not assigned to a specific release yet.

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
