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

# Single test by name
cargo test test_read_line

# Tests for specific crate
cargo test -p logmancer-core

# With output
cargo test -- --nocapture
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
