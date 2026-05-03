# Containers

This document describes Logmancer at C4 Level 2: the major runtime and source-code containers that make up the system.

## C4 Level 2: Container Diagram

```mermaid
flowchart LR
    user["Developer / Operator<br/>Inspects and searches large log files"]

    terminal["Terminal"]
    browser["Web Browser"]
    desktop_os["Desktop Operating System"]
    filesystem[("Local File System<br/>Stores log files selected by the user")]

    subgraph logmancer["Logmancer"]
        tui["logmancer-tui<br/>Rust / crossterm<br/>Interactive terminal log viewer with keyboard navigation similar to less"]
        web["logmancer-web<br/>Rust / Leptos / Axum<br/>Web application for viewing logs through a browser"]
        desktop["logmancer-desktop<br/>Rust / Tauri<br/>Desktop shell that packages the web experience as a native application"]
        core["logmancer-core<br/>Rust library<br/>Shared file-processing logic for indexing, reading, searching, filtering, and parsing logs"]
    end

    user --> terminal
    user --> browser
    user --> desktop_os

    terminal -->|Starts and controls via keyboard input| tui
    browser -->|Requests UI and actions| web
    desktop_os -->|Runs| desktop
    desktop -->|Packages / reuses via Tauri webview| web

    tui -->|Uses Rust API| core
    web -->|Uses Rust API / server functions| core
    desktop -->|Indirectly uses through web module| core

    core -->|Reads log files via File I/O| filesystem
```

## Containers

### logmancer-core

Shared Rust library containing the core log-processing capabilities. It owns the reusable logic for reading from disk, indexing files, searching, filtering, and parsing log lines.

### logmancer-tui

Interactive terminal application for inspecting logs directly from a shell. It depends on `logmancer-core` for file and log operations and focuses on terminal rendering, keyboard input, and viewer navigation.

### logmancer-web

Leptos and Axum-based web application. It exposes Logmancer through a browser-based interface while reusing `logmancer-core` for the underlying log operations.

### logmancer-desktop

Tauri-based desktop application. It packages the web experience as a native desktop app and leverages the same core behavior through the web module integration.

## Key Dependency Direction

Application surfaces depend on `logmancer-core`; the core library does not depend on any UI container. This keeps log-processing behavior reusable across terminal, web, and desktop distributions.
