# Changelog

All notable changes to Logmancer will be documented in this file.

The format is inspired by [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project uses semantic versioning.

## [Unreleased]

### Changed

- Desktop development mode can now run against an external `cargo leptos watch` server without embedding the Leptos/Axum server in the Tauri crate, reducing `tauri dev --no-default-features` compile work.

### Fixed

- Desktop external-server mode now preserves desktop runtime detection across hydration and log navigation, so native file opening and drag/drop continue to work while avoiding duplicate Home file buttons.
- Server-root file opening now accepts native-picker absolute paths only after canonicalizing them inside `LOGMANCER_SERVER_FILE_ROOT`.

## [0.3.0] - 2026-07-07

### Added

- Shared core search engine with reusable `PageResult` search metadata (`PageSearchResult` + page-scoped match spans with multiple occurrences per line), less-style wrap navigation (`next`/`previous`), and stable match identity for frontend adapters.
- Search API endpoints for web/desktop thin adapters (`apply-search`, `search-next`, `search-previous`, `search-status`, `clear-search`) backed directly by core semantics.
- Web/desktop bottom search panel opened with `/` or `Ctrl+F`, backed by the shared search API and current-page match highlighting.
- Web/desktop search match navigation with `n` and `N`, matching core/TUI wrap behavior and keeping the selected match visible (#23).
- TUI search prompt opened with `/`, including submit/cancel handling and `n`/`N` next/previous match navigation using core behavior.
- Search match highlighting across Web/Desktop main and filtered views plus TUI visible lines, with a distinct current-match state (#24).
- Search status display across TUI and Web/Desktop now shows the active query, current match position, total matches, and no-match feedback (#25).
- Desktop Home can open local log files through a native file picker or native drag/drop without requiring `LOGMANCER_SERVER_FILE_ROOT` (#61).

### Changed

- Milestone version moved to `0.3.0` across crate metadata, lockfiles, desktop Tauri config, and README development version reference.
- Web/desktop log scrolling now uses gentler increments for arrow keys, native scrollbar movement, and mouse wheel input.
- Web/desktop filter controls now include a visible Search button and use a tighter monospace presentation.
- Web/desktop auto-scroll status is more compact and highlights the active state.
- Refactored web viewer internals to separate line decoration, line rendering, navigation decisions, main-pane search helpers, and browser API client code for smaller reviewable responsibilities.

### Fixed

- Web/desktop bottom search panel now left-aligns the search input and lets it expand across the available width.

## [0.2.0] - 2026-05-15

### Added

- Project roadmap in `ROADMAP.md`.
- Local web file upload flow from the Home screen (#10).
- Selected line highlighting in the web/desktop main viewer and filter panel (#15).
- Filter result selection now synchronizes with the main viewer, selecting and revealing the matching original log line (#16).
- Active log panel indication for the web/desktop viewer and filter panel (#17).
- Added a visible auto-scroll/follow mode indicator in the web/desktop UI (#18).
- Root-scoped server file browser for Logmancer Web, enabled by `LOGMANCER_SERVER_FILE_ROOT`, with Spotlight-style navigation, local filtering, root-bound backend validation, and text-file opening (#40).
- Portable release packaging with versioned README/launchers, simplified Linux/Windows artifact names, and tag-triggered GitHub Release asset publishing (#43, #45).

### Changed

- Split CI testing and release-build packaging workflows, with non-blocking dependency caches and the `windows-2025-vs2026` Windows runner (#28, #31, #33).
- Commit/PR guidance now requires an explicit `CHANGELOG.md` decision for release-relevant changes.
- Desktop and web release branding now use the Logmancer app name and the desktop build uses the `com.ignsabbag.logmancer` application identifier (#47).
- Windows desktop portable launcher now starts the GUI executable without keeping the launcher console open (#47).

### Fixes

- Release web builds now hydrate correctly and initialize backend logging reliably.
- Web keyboard navigation now supports repeated arrow/page movement plus `g`, `G`, and `f` shortcuts.
- Web main viewer keeps auto-scroll status scoped to the main pane and avoids filter-panel scroll/follow coupling.
- Web/desktop log scrolling now preserves accumulated mouse wheel and trackpad intent, avoiding stale debounced viewport updates that made fast scrolling feel like it moved backwards (#19).
- Large-file indexing no longer blocks main log reads while scanning new line offsets, keeping the UI responsive when applying filters during indexing (#30).
- Release builds now set a higher Leptos recursion limit so GitHub release packaging can compile the web UI reliably (#42).

## [0.1.0] - 2026-05-05

### Added

- Optimized disk-backed log file visualization and indexing.
- `less`-style navigation, including line/page movement and `g` / `G` jumps.
- Follow mode with `f` in TUI and web.
- Regex-based filtering.
- Filter panel in web/desktop.
- Rust workspace structure with core, TUI, web, and desktop crates.

### Known gaps

- Web follow mode works, but does not yet expose a clear visual indicator.
- Search, visual rules, structured parsing, and multi-file workspace improvements are planned but not part of `0.1.0`.

[Unreleased]: https://github.com/ignsabbag/logmancer/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/ignsabbag/logmancer/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/ignsabbag/logmancer/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/ignsabbag/logmancer/releases/tag/v0.1.0
