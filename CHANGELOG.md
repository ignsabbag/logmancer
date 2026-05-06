# Changelog

All notable changes to Logmancer will be documented in this file.

The format is inspired by [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project uses semantic versioning.

## [Unreleased]

### Added

- Project roadmap in `ROADMAP.md`.
- Selected line highlighting in the web/desktop main viewer and filter panel (#15).
- Filter result selection now synchronizes with the main viewer, selecting and revealing the matching original log line (#16).

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
