Logmancer portable package

This is a portable app package. No Logmancer installation is required.

Quick start:
- Desktop: run logmancer with an optional log file path.
- Web: run logmancer web [options], then open the printed local URL in your browser.
- TUI: run the TUI binary with a log file path.

Examples:
- Linux desktop: ./logmancer /path/to/logfile.log
- Linux web: ./logmancer web [options]
- Linux TUI: ./logmancer tui /path/to/logfile.log
- Direct Linux TUI: ./logmancer-tui /path/to/logfile.log
- Windows desktop: logmancer.exe C:\path\to\logfile.log
- Windows web: logmancer.exe web [options]
- Windows TUI: logmancer.exe tui C:\path\to\logfile.log
- Direct Windows TUI: logmancer-tui.exe C:\path\to\logfile.log

The launcher and legacy run-desktop/run-web wrappers use the bundled site/ directory when no LEPTOS runtime override is set.
For Web, use --bind <socket-address> and --file-root <path>; CLI values override LOGMANCER_BIND_ADDR and LOGMANCER_SERVER_FILE_ROOT, which override safe defaults.
Runtime logs are written to the logs/ directory next to the launchers.
