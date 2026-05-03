# System Context

This document describes Logmancer at C4 Level 1: the system in its environment and the people or external systems that interact with it.

## Scope

Logmancer is a lightweight, cross-platform log viewer written in Rust. It helps developers and operators inspect large log files efficiently through terminal, web, and desktop interfaces.

## C4 Level 1: System Context

```mermaid
flowchart LR
    user["Developer / Operator<br/>Inspects and searches large log files"]

    terminal["Terminal<br/>Runs the terminal UI"]
    browser["Web Browser<br/>Runs the web UI"]
    desktop_os["Desktop Operating System<br/>Runs the desktop app shell"]

    logmancer["Logmancer<br/>Cross-platform log viewer for reading, navigating, filtering, searching, and parsing large log files"]
    filesystem[("Local File System<br/>Stores log files selected by the user")]

    user -->|Uses keyboard| terminal
    user -->|Uses HTTP/UI| browser
    user -->|Uses native desktop UI| desktop_os

    terminal -->|Launches terminal viewer| logmancer
    browser -->|Interacts with web viewer| logmancer
    desktop_os -->|Hosts desktop viewer| logmancer

    logmancer -->|Reads log files via File I/O| filesystem
```

## Main Responsibilities

- Read large log files efficiently from disk.
- Navigate logs with viewer-like controls, including line scrolling and jump-to-start/end behavior.
- Search and filter log content using regular expressions.
- Parse log lines into structured columns when a supported or custom pattern is available.
- Provide multiple delivery surfaces: terminal UI, web app, and desktop app.

## Out of Scope

- Centralized log ingestion or storage.
- Remote log collection agents.
- Authentication, authorization, or multi-user management.
- Long-term persistence of log analysis state.
