# AGENTS.md - Logmancer Development Guide

## Overview

Logmancer is a Rust workspace with multiple crates: `logmancer-core` (core logic), `logmancer-tui` (terminal UI app), `logmancer-web` (Leptos web app), and `logmancer-desktop` (Tauri desktop app).

---

## Build Commands

### Full Project
```bash
# Build entire workspace
cargo build --release

# Run all tests
cargo test

# Run clippy lints
cargo clippy -- -D warnings
```

### Individual Crates
```bash
# CLI
cargo build --release -p logmancer-tui
cargo run --bin logmancer-tui -- /path/to/logfile.log

# Core library
cargo build --release -p logmancer-core

# Web (Leptos)
cargo leptos build --release --project logmancer-web
cargo leptos watch --project logmancer-web    # Dev server with hot reload

# Desktop (Tauri)
export LEPTOS_OUTPUT_NAME=logmancer-web
cargo tauri build --no-bundle
cargo tauri dev                              # Dev mode
```

### Running Tests
```bash
# All tests in workspace
cargo test

# Fast local feedback without the heavyweight Tauri desktop crate
cargo test --workspace --exclude logmancer-desktop

# SSR web tests when changing Leptos server-side behavior
cargo test -p logmancer-web --features ssr --lib

# Full desktop validation, including the embedded server
cargo test -p logmancer-desktop

# Single test by name
cargo test test_read_line

# Tests for specific crate
cargo test -p logmancer-core

# With output
cargo test -- --nocapture

# Reclaim debug and test artifacts without removing release builds
cargo clean --profile dev
```

---

## Code Style Guidelines

### Project Structure
- Use `mod` to define modules (one per file or multiple in `mod.rs`)
- Use `pub use` for public re-exports from lib.rs
- Organize: `src/lib.rs` -> `pub mod component_name;` -> `src/component_name/mod.rs`

### Naming Conventions
- **Structs/Enums**: `PascalCase` (e.g., `LogReader`, `FileInfo`)
- **Functions/Variables**: `snake_case` (e.g., `read_page`, `total_lines`)
- **Constants**: `SCREAMING_SNAKE_CASE`
- **Files**: `snake_case.rs` (e.g., `file_ops/read.rs`)

### Imports
- Group imports by:
  1. Standard library (`std::io`, `std::sync`)
  2. External crates (`crate::`, `log::`, `leptos::`)
- Use absolute paths: `crate::models::FileInfo`
- Avoid wildcards except for test modules

### Types and Ownership
- Use `&str` over `String` for function parameters when possible
- Use `io::Result<T>` for file operations (equivalent to `Result<T, io::Error>`)
- Use `anyhow::Result<T>` for application-level errors
- Prefer `RwLock` for shared mutable state in core library
- Use Serde's `#[derive(Serialize, Deserialize)]` for serialization

### Error Handling
- File operations: Return `io::Result<T>`
- Application logic: Use `anyhow` for contextual errors
- Leptos: Use `Result<T, String>` for server functions
- Always propagate errors with `?` operator

### Leptos/Web Patterns
- Server functions: `#[server]` macro with `Result<T, String>`
- Components: `fn component_name() -> impl IntoView`
- Use `use crate::components::*` in hydrate
- State: Use Leptos signals (`create_signal`, `create_resource`)

### Tauri/Desktop
- Build requires SSR leptos feature: `features = ["ssr"]`
- Use `tauri-plugin-opener` for external links

### Testing
- Use `#[cfg(test)] mod tests { ... }` for inline tests
- Use `#[test]` for test functions
- Prefer temp files for file operation tests
- Clean up test files in test body

### Changelog
- Update `CHANGELOG.md` before committing relevant user-facing, release, packaging, workflow, or behavior changes.
- Keep changelog entries concise and grouped under `[Unreleased]` during development and beta releases. Move them into a dated release section only when preparing a stable release; keep the `[Unreleased]` comparison based on the last stable version until then.
- Beta version bumps may leave `CHANGELOG.md` unchanged when all relevant changes are already under `[Unreleased]`; explain this decision in the commit/PR summary.
- If a change is purely internal and not release-note-worthy, leave `CHANGELOG.md` unchanged and call that out in the commit/PR summary.

### Architecture Decisions
- At the start of every task, read `docs/adr/README.md`, the architectural decision index and workflow.
- Before proposing or changing behavior covered by an ADR, read the full relevant records and any related ADRs needed to understand their constraints. Do not read every ADR by default.
- Accepted ADRs are current architectural constraints. Proposed ADRs are discussion context, not approved decisions; superseded ADRs are historical context.
- If a solution would contradict an accepted ADR, identify the conflict and agree on a new decision before implementing it. Agents must not accept proposed ADRs without explicit approval.
- Do not rewrite an accepted ADR's historical decision. Create a new ADR that references or supersedes it; only status and relationship metadata may be updated to record supersession.
- When adding an ADR or changing its status or relationships, update `docs/adr/README.md` in the same change.

---

## Dependencies

### Core Crates
- `log = "0.4"` - Logging facade
- `memmap2` - Memory-mapped files
- `regex` - Regex filtering
- `dashmap` - Concurrent HashMap
- `uuid` - File IDs

### Web Crates
- `leptos` - Web framework
- `leptos_router` - Routing
- `axum` - HTTP server
- `serde` - Serialization
- `wasm-bindgen` - WASM binding

### Desktop Crates
- `tauri` - Desktop framework

---

## Common Tasks

### Adding a New Module
1. Create `src/new_module.rs`
2. Add `mod new_module;` in `lib.rs`
3. Add `pub use new_module::NewModule;` if public

### Adding a New API Endpoint (Web)
1. Add handler in `src/api/`
2. Register in `src/api/config.rs`

### Running the Application
```bash
# CLI viewer
cargo run --bin logmancer-tui -- ./test.log

# Web server
cargo leptos watch --project logmancer-web

# Desktop
cargo tauri dev
```

---

## Project Skills

- `skills/create-issue/SKILL.md` — Use to draft or create GitHub issues in English, check duplicates, and select existing `type:*` and `area:*` labels. Publication and implementation approval are separate authorizations; do not change existing issues without an explicit request.
- `skills/prepare-release/SKILL.md` — Use for beta and stable releases, version bumps, and release tags. Follow its changelog policy, validate before tagging, and respect branch protections through PRs without bypassing required checks. Local-only preparation does not authorize commits or publication.
- `skills/rust-quality-gate/SKILL.md` — OpenCode-compatible skill for commit/PR preparation. It requires `cargo fmt` and `cargo clippy --workspace -- -D warnings` before creating a commit or PR.
- The Rust quality gate skill also requires an explicit `CHANGELOG.md` decision before committing relevant user-facing, release, packaging, workflow, or behavior changes.
