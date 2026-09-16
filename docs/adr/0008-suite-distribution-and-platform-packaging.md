# ADR 0008: Suite distribution and platform packaging

## Status

Proposed

## Decision

Logmancer is distributed as one suite containing its Desktop, Web, and TUI
variants. The initial installed distribution targets Windows through an NSIS
installer and Debian-based Linux through a DEB package.

A separate `logmancer-launcher` workspace crate produces the public
`logmancer` executable. The launcher starts one of the separately packaged
Desktop, Web, and TUI executables; it does not contain application logic from
any variant. The variant executables remain directly invokable.

Desktop is the default experience. Explicit `desktop`, `web`, and `tui`
subcommands always select the requested variant. On Windows, `logmancer [file...]`
always starts Desktop. On Linux, `logmancer <file> [file...]` starts Desktop when the
current session exposes a graphical display and otherwise starts TUI when the
process is attached to an interactive terminal. It reports an actionable error
instead of selecting a variant when neither condition is met. Bare `logmancer`
requires a graphical session because the TUI currently requires a file.

The generated Leptos `site/` directory is installed as an application resource.
Desktop resolves that resource through Tauri, while standalone Web resolves its
installed assets relative to its own executable. A valid resource contains the
generated `pkg/logmancer-web.css`, `pkg/logmancer-web.js`, and
`pkg/logmancer-web.wasm` artifacts; SSR generates HTML dynamically and does not
require `site/index.html`. The launcher supplies matching Leptos defaults only
when the values are unset or empty and a valid sibling `site/` exists. Existing
environment variables remain explicit overrides for portable and development
use. Runtime logs identify the source of resolved parameters without recording
their values.

## Context

Current releases publish portable ZIP archives containing `logmancer-desktop`,
`logmancer-web`, `logmancer-tui`, generated `site/` assets, and platform
launchers. Desktop embeds an SSR server; the standalone Web variant serves the
same generated assets. Installed Desktop and Web runtimes resolve those assets
without requiring callers to set `LEPTOS_SITE_ROOT`, while portable launchers
retain the variable as an explicit override.

The project needs native installation for the full suite without losing the
portable distribution. A single monolithic executable is not required: Tauri,
the terminal UI, and the standalone server have incompatible platform and
runtime concerns, particularly on Windows where Desktop suppresses the console.

## Decisions

| Area | Decision |
|---|---|
| Distribution unit | Ship Desktop, Web, TUI, launcher, and `site/` together. |
| Windows package | Use Tauri's NSIS bundle. |
| Linux package | Use Tauri's DEB bundle for Debian-based distributions. |
| Launcher ownership | Build `logmancer` from an independent `logmancer-launcher` workspace crate with no application logic from a variant. |
| Public command | Provide `logmancer [file...]`, `logmancer desktop [file...]`, `logmancer web [options]`, and `logmancer tui <file> [file...]`. |
| Explicit selection | Explicit variant subcommands always override default selection. |
| Windows default | `logmancer [file...]` always starts Desktop. |
| Linux default | `logmancer <file> [file...]` starts Desktop in a graphical session or TUI in a non-graphical interactive terminal. Bare `logmancer` requires a graphical session. |
| Web selection | Start standalone Web only through the explicit `web` subcommand or its direct executable. |
| Variant executables | Keep `logmancer-desktop`, `logmancer-web`, and `logmancer-tui` directly runnable. |
| Portable and Windows layout | Keep the launcher, variant executables, and `site/` in one suite directory. |
| Linux layout | Install the suite under `/usr/lib/logmancer` and expose the launcher and variant executables through `/usr/bin`. |
| Web startup | Start manually and bind to loopback by default. |
| File exposure | Keep `LOGMANCER_SERVER_FILE_ROOT` optional and explicitly configured. |
| Web configuration | Apply `--bind`/`--file-root`, then environment variables, then safe defaults; do not apply standalone Web options to Desktop. |
| Web assets | Install generated SSR `site/pkg/logmancer-web.{css,js,wasm}` assets as a resource and resolve their installed path at runtime. |
| Runtime configuration logging | Log whether each runtime parameter came from CLI, environment, installed resources, or a default without logging values. |
| Portable releases | Retain portable ZIP archives during the transition. |
| Arch Linux | Prepare a source-built `logmancer` AUR `PKGBUILD` after the installed layout is stable. |

## Consequences

- Release automation must build and publish NSIS and DEB artifacts in addition
  to the portable ZIP archives.
- Installed Desktop and Web startup must not rely on a caller to set
  `LEPTOS_SITE_ROOT`.
- Launcher defaults must not replace explicit Leptos environment overrides.
- The launcher is a user-facing command contract and needs argument, error, and
  platform-specific integration tests.
- Linux default-selection tests must cover graphical, non-graphical interactive,
  and non-interactive sessions. Detection describes the current session, not
  whether the machine has a desktop environment installed.
- Windows always defaults to Desktop; server and headless use remains available
  through explicit `tui` and `web` subcommands.
- Native packages must expose `logmancer` on `PATH` while keeping the real suite
  executables and assets together. Desktop shortcuts should invoke
  `logmancer-desktop` directly.
- AUR packaging can install the same suite layout rather than inventing a
  separate application model.
- A monolithic executable remains possible later, but is not a dependency of
  platform packaging.

## Out of scope

- A monolithic single-process executable.
- Running the Web variant as a system service.
- Automatically starting Web when no graphical or interactive terminal session
  is available.
- Falling back to TUI after Desktop starts and then fails; such failures must
  remain visible instead of being treated as environment detection.
- MSI, RPM, Flatpak, Snap, or Microsoft Store packages.
- Automatic updates and installer signing.
- AUR publication automation or a prebuilt `logmancer-bin` package.

## Related work

- GitHub epic: [#104](https://github.com/ignsabbag/logmancer/issues/104).
- ADR 0004 defines the native configuration-directory ownership model used by
  installed variants.

## Verification checklist

- [ ] NSIS installs the complete suite and supports uninstallation.
- [ ] DEB installs the complete suite with a desktop-menu entry.
- [ ] Explicit launcher subcommands start the requested variant on every
  supported platform.
- [ ] `logmancer [file...]` starts Desktop on Windows.
- [ ] `logmancer <file> [file...]` starts Desktop in a Linux graphical session and TUI in
  a Linux non-graphical interactive terminal.
- [ ] Bare `logmancer` in a Linux non-graphical session reports an actionable
  error instead of starting Web or TUI.
- [ ] Non-interactive launcher use does not start TUI implicitly.
- [ ] Each variant executable starts directly after installation.
- [ ] Desktop and Web resolve installed `site/` assets without manual
  environment configuration.
- [ ] Web remains loopback-only unless explicitly configured otherwise.
- [ ] Portable ZIP archives retain their documented behavior.
