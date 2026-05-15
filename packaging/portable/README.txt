Logmancer portable package

This is a portable app package. No Logmancer installation is required.

Quick start:
- Web: run the web launcher, then open the printed local URL in your browser.
- Desktop: run the desktop launcher.
- TUI: run the TUI binary with a log file path.

Examples:
- Linux web: ./run-web.sh /path/to/logfile.log
- Linux desktop: ./run-desktop.sh /path/to/logfile.log
- Linux TUI: ./logmancer-tui /path/to/logfile.log
- Windows web: run-web.cmd C:\path\to\logfile.log
- Windows desktop: run-desktop.cmd C:\path\to\logfile.log
- Windows TUI: logmancer-tui.exe C:\path\to\logfile.log

The web and desktop launchers set the Leptos runtime environment to use the bundled site/ directory.
Runtime logs are written to the logs/ directory next to the launchers.
