# Changelog

All notable changes to Logmancer will be documented in this file.

The format is inspired by [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project uses semantic versioning.

## [Unreleased]

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

[Unreleased]: https://github.com/ignsabbag/logmancer/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/ignsabbag/logmancer/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/ignsabbag/logmancer/releases/tag/v0.1.0
