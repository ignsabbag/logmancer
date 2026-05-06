# logmancer

[![Test](https://github.com/ignsabbag/logmancer/actions/workflows/test.yml/badge.svg)](https://github.com/ignsabbag/logmancer/actions/workflows/test.yml)
[![Build](https://github.com/ignsabbag/logmancer/actions/workflows/build.yml/badge.svg)](https://github.com/ignsabbag/logmancer/actions/workflows/build.yml)

A lightweight, cross-platform log viewer written in Rust. Designed for efficiency and speed, Logmancer reads directly from disk and handles very large log files with ease.

Current development version: `0.2.0`.

---

## Table of Contents

* [Features](#features)
* [Architecture & Modules](#architecture--modules)
* [Getting Started](#getting-started)
  * [Prerequisites](#prerequisites)
  * [Building from Source](#building-from-source)
* [Usage](#usage)
  * [logmancer-tui](#logmancer-tui)
  * [logmancer-web](#logmancer-web)
  * [logmancer-desktop](#logmancer-desktop)
  * [Controls](#controls)
* [Configuration](#configuration)
* [Roadmap](#roadmap)
* [Contributing](#contributing)
* [License](#license)

---

## Features

* **Efficient disk-backed reading** of very large files.
* **Optimized file indexing** for fast navigation.
* **`less`-style navigation** with keyboard shortcuts such as `g`, `G`, page movement, and follow mode.
* **Regex-based filtering** in web/desktop with results navigable in a separate panel.
* **TUI, web, and desktop frontends** built on a shared core.

*Note: The project is in its early stages of development.*

Planned features include search, visual rules, multi-file workspace improvements, and structured log parsing. See [ROADMAP.md](ROADMAP.md) for details.

---

## Architecture & Modules

Logmancer is structured as a multi-crate workspace:

* **logmancer-core**: Core logic for file indexing, reading, and searching.
* **logmancer-tui**: Terminal UI application.
* **logmancer-web**: Web application using Leptos and Axum.
* **logmancer-desktop**: Desktop application leveraging the web module via Tauri.

---

## Getting Started

### Prerequisites

* [Rust toolchain](https://www.rust-lang.org/tools/install) (stable)

### Building from Source

1. Clone the repository:
   ```sh
   git clone https://github.com/ignsabbag/logmancer.git
   cd logmancer
   ```

2. Build the entire project:
   ```sh
   cargo build --release
   ```

3. Build the web module:
   ```sh
   cargo leptos build --release --project logmancer-web
   ```

4. Build the desktop module:
   ```sh
   export LEPTOS_OUTPUT_NAME=logmancer-web
   cargo tauri build --no-bundle
   ```

---

## Usage

### logmancer-tui

Launch the terminal log viewer by specifying a log file path:
```sh
cargo run --bin logmancer-tui -- /path/to/your/logfile.log
```

### logmancer-web

Run the development web server with Leptos:
```sh
cargo leptos watch --project logmancer-web
```

### logmancer-desktop

Launch the desktop app using Tauri:
```sh
cargo tauri dev
```

### Controls (similar to `less`)

* **Arrow keys**: Scroll up/down line by line.
* **Page Up / Page Down**: Scroll by page.
* **g**: Go to beginning of file.
* **G**: Go to end of file.
* **f** / **F**: Toggle follow mode (like `tail -f`), only works at EOF.
* **q**: Quit (CLI only).

---

## Configuration

No additional configuration is required at this time. Future modules may introduce config files or environment variables.

---

## Roadmap

See [ROADMAP.md](ROADMAP.md) for planned releases and future ideas.

Release history is tracked in [CHANGELOG.md](CHANGELOG.md).

---

## Contributing

Contributions are welcome! To get started:

1. Fork the repository.
2. Create a feature branch: `git checkout -b feature/YourFeature`.
3. Commit your changes and push to your fork.
4. Open a Pull Request describing your changes.

Please adhere to the existing code style and include tests where applicable.

---

## License

This project is licensed under the [MIT License](LICENSE).

---

*Built with ❤️ in Rust.*
