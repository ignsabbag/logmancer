# logmancer

[![CI](https://github.com/ignsabbag/logmancer/actions/workflows/rust.yml/badge.svg)](https://github.com/ignsabbag/logmancer/actions/workflows/rust.yml)

A lightweight, cross-platform log viewer written in Rust. Designed for efficiency and speed, Logmancer reads directly from disk and handles very large log files with ease.

---

## Table of Contents

* [Features](#features)
* [Architecture & Modules](#architecture--modules)
* [Getting Started](#getting-started)
  * [Prerequisites](#prerequisites)
  * [Building from Source](#building-from-source)
* [Usage](#usage)
  * [logmancer-cli](#logmancer-cli)
  * [logmancer-web](#logmancer-web)
  * [logmancer-desktop](#logmancer-desktop)
  * [Controls](#controls)
* [Configuration](#configuration)
* [Contributing](#contributing)
* [License](#license)

---

## Features

* **Efficient disk-backed reading** of very large files.
* **Regex-based filtering** with results navigable in a separate panel.
* **Colorized search** highlighting for easier log inspection.
* **Line parsing** by pattern (e.g., Log4j) or custom regex, displayed in columns each with its own filter.

*Note: The project is in its early stages of development.*

---

## Architecture & Modules

Logmancer is structured as a multi-crate workspace:

* **logmancer-core**: Core logic for file indexing, reading, and searching.
* **logmancer-cli**: Command-line interface application.
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

### logmancer-cli

Launch the CLI log viewer by specifying a log file path:
```sh
cargo run --bin logmancer-cli -- /path/to/your/logfile.log
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
