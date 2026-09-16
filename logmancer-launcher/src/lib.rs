use std::env;
use std::ffi::OsString;
use std::fmt;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Linux,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Session {
    pub graphical: bool,
    pub interactive_terminal: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    Desktop,
    Web,
    Tui,
}

impl Variant {
    fn executable_name(self) -> &'static str {
        match self {
            Self::Desktop => "logmancer-desktop",
            Self::Web => "logmancer-web",
            Self::Tui => "logmancer-tui",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchRequest {
    pub variant: Variant,
    pub arguments: Vec<OsString>,
}

#[derive(Debug)]
pub enum LauncherError {
    Usage(String),
    MissingDisplay,
    NonInteractive,
    ExecutableDirectory(io::Error),
    MissingExecutable {
        variant: Variant,
        directory: PathBuf,
    },
    Start {
        executable: PathBuf,
        source: io::Error,
    },
}

impl fmt::Display for LauncherError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) => write!(formatter, "{message}"),
            Self::MissingDisplay => write!(
                formatter,
                "A graphical display is required for `logmancer`. Use `logmancer tui <file>` in a terminal or `logmancer web [options]` to start the web server."
            ),
            Self::NonInteractive => write!(
                formatter,
                "No graphical display or interactive terminal is available. Use `logmancer desktop [file...]`, `logmancer tui <file> [file...]`, or `logmancer web [options]` explicitly."
            ),
            Self::ExecutableDirectory(source) => write!(
                formatter,
                "Could not resolve the Logmancer launcher directory: {source}"
            ),
            Self::MissingExecutable { variant, directory } => write!(
                formatter,
                "Could not find {} next to the launcher in {}. Reinstall the complete Logmancer suite.",
                variant.executable_name(),
                directory.display()
            ),
            Self::Start { executable, source } => write!(
                formatter,
                "Could not start {}: {source}",
                executable.display()
            ),
        }
    }
}

impl std::error::Error for LauncherError {}

pub fn current_platform() -> Platform {
    if cfg!(windows) {
        Platform::Windows
    } else if cfg!(target_os = "linux") {
        Platform::Linux
    } else {
        Platform::Other
    }
}

pub fn current_session() -> Session {
    Session {
        graphical: env::var_os("DISPLAY").is_some_and(|value| !value.is_empty())
            || env::var_os("WAYLAND_DISPLAY").is_some_and(|value| !value.is_empty()),
        interactive_terminal: io::stdin().is_terminal() && io::stdout().is_terminal(),
    }
}

pub fn parse_request(
    arguments: &[OsString],
    platform: Platform,
    session: Session,
) -> Result<LaunchRequest, LauncherError> {
    let usage = || {
        LauncherError::Usage("Usage: logmancer [file...] | logmancer desktop [file...] | logmancer web [options] | logmancer tui <file> [file...]".to_string())
    };

    if let Some((variant, remaining)) = explicit_variant(arguments) {
        if variant == Variant::Tui && remaining.is_empty() {
            return Err(usage());
        }
        return Ok(LaunchRequest {
            variant,
            arguments: remaining.to_vec(),
        });
    }

    match platform {
        Platform::Windows => Ok(LaunchRequest {
            variant: Variant::Desktop,
            arguments: arguments.to_vec(),
        }),
        Platform::Linux => match (
            arguments.is_empty(),
            session.graphical,
            session.interactive_terminal,
        ) {
            (true, true, _) | (false, true, _) => Ok(LaunchRequest {
                variant: Variant::Desktop,
                arguments: arguments.to_vec(),
            }),
            (false, false, true) => Ok(LaunchRequest {
                variant: Variant::Tui,
                arguments: arguments.to_vec(),
            }),
            (true, false, _) => Err(LauncherError::MissingDisplay),
            (false, false, false) => Err(LauncherError::NonInteractive),
        },
        Platform::Other => Ok(LaunchRequest {
            variant: Variant::Desktop,
            arguments: arguments.to_vec(),
        }),
    }
}

fn explicit_variant(arguments: &[OsString]) -> Option<(Variant, &[OsString])> {
    let variant = match arguments.first().and_then(|argument| argument.to_str()) {
        Some("desktop") => Variant::Desktop,
        Some("web") => Variant::Web,
        Some("tui") => Variant::Tui,
        _ => return None,
    };
    Some((variant, &arguments[1..]))
}

pub fn resolve_sibling_executable(
    directory: &Path,
    variant: Variant,
    platform: Platform,
) -> Result<PathBuf, LauncherError> {
    let name = variant.executable_name();
    let executable = if platform == Platform::Windows {
        directory.join(format!("{name}.exe"))
    } else {
        directory.join(name)
    };
    executable
        .exists()
        .then_some(executable)
        .ok_or_else(|| LauncherError::MissingExecutable {
            variant,
            directory: directory.to_path_buf(),
        })
}

pub fn launch(
    request: LaunchRequest,
    directory: &Path,
    platform: Platform,
) -> Result<ExitStatus, LauncherError> {
    let executable = resolve_sibling_executable(directory, request.variant, platform)?;
    let mut command = Command::new(&executable);
    command.args(request.arguments);
    apply_leptos_defaults(&mut command, request.variant, directory);
    #[cfg(windows)]
    if platform == Platform::Windows && request.variant == Variant::Desktop {
        detach_owned_console();
    }
    command
        .status()
        .map_err(|source| LauncherError::Start { executable, source })
}

#[cfg(any(windows, test))]
fn should_detach_console(console_processes: &[u32], launcher_process_id: u32) -> bool {
    console_processes == [launcher_process_id]
}

#[cfg(windows)]
fn detach_owned_console() {
    use windows_sys::Win32::System::Console::{FreeConsole, GetConsoleProcessList};

    let mut console_processes = [0];
    let process_count = unsafe {
        GetConsoleProcessList(
            console_processes.as_mut_ptr(),
            console_processes.len() as u32,
        )
    };

    if process_count == console_processes.len() as u32
        && should_detach_console(&console_processes, std::process::id())
    {
        unsafe { FreeConsole() };
    }
}

fn apply_leptos_defaults(command: &mut Command, variant: Variant, directory: &Path) {
    if variant == Variant::Tui {
        return;
    }

    command.env_remove("LOGMANCER_LEPTOS_OUTPUT_NAME_SOURCE");
    command.env_remove("LOGMANCER_LEPTOS_SITE_ROOT_SOURCE");

    if env::var_os("LEPTOS_OUTPUT_NAME").is_none_or(|value| value.is_empty()) {
        command.env("LEPTOS_OUTPUT_NAME", "logmancer-web");
        command.env("LOGMANCER_LEPTOS_OUTPUT_NAME_SOURCE", "default");
        eprintln!("Resolved LEPTOS_OUTPUT_NAME from default");
    } else {
        eprintln!("Resolved LEPTOS_OUTPUT_NAME from environment");
    }

    if env::var_os("LEPTOS_SITE_ROOT").is_none_or(|value| value.is_empty()) {
        let site_root = directory.join("site");
        if is_valid_site_root(&site_root)
            && let Ok(site_root) = site_root.canonicalize()
        {
            command.env("LEPTOS_SITE_ROOT", site_root);
            command.env("LOGMANCER_LEPTOS_SITE_ROOT_SOURCE", "installed_resource");
            eprintln!("Resolved LEPTOS_SITE_ROOT from installed resource");
        } else {
            eprintln!("Resolved LEPTOS_SITE_ROOT from default");
        }
    } else {
        eprintln!("Resolved LEPTOS_SITE_ROOT from environment");
    }
}

fn is_valid_site_root(site_root: &Path) -> bool {
    let package_directory = site_root.join("pkg");
    package_directory.join("logmancer-web.css").is_file()
        && package_directory.join("logmancer-web.js").is_file()
        && package_directory.join("logmancer-web.wasm").is_file()
}

pub fn run_from_env() -> Result<(), LauncherError> {
    let arguments: Vec<OsString> = env::args_os().skip(1).collect();
    let request = parse_request(&arguments, current_platform(), current_session())?;
    let launcher = env::current_exe().map_err(LauncherError::ExecutableDirectory)?;
    let directory = launcher.parent().ok_or_else(|| {
        LauncherError::ExecutableDirectory(io::Error::other(
            "launcher path has no parent directory",
        ))
    })?;
    let status = launch(request, directory, current_platform())?;
    propagate_exit_status(status)
}

fn propagate_exit_status(status: ExitStatus) -> Result<(), LauncherError> {
    if status.success() {
        return Ok(());
    }
    #[cfg(unix)]
    if let Some(signal) = std::os::unix::process::ExitStatusExt::signal(&status) {
        unsafe { libc::raise(signal) };
    }
    std::process::exit(status.code().unwrap_or(1));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    const GRAPHICAL: Session = Session {
        graphical: true,
        interactive_terminal: false,
    };
    const HEADLESS_TERMINAL: Session = Session {
        graphical: false,
        interactive_terminal: true,
    };
    const HEADLESS_NON_INTERACTIVE: Session = Session {
        graphical: false,
        interactive_terminal: false,
    };

    #[test]
    fn explicit_subcommands_take_precedence_and_preserve_arguments() {
        assert_eq!(
            parse_request(
                &args(&["web", "--port", "8080"]),
                Platform::Linux,
                HEADLESS_NON_INTERACTIVE
            )
            .unwrap(),
            LaunchRequest {
                variant: Variant::Web,
                arguments: args(&["--port", "8080"])
            }
        );
        assert_eq!(
            parse_request(
                &args(&["desktop", "file with spaces.log"]),
                Platform::Linux,
                HEADLESS_TERMINAL
            )
            .unwrap(),
            LaunchRequest {
                variant: Variant::Desktop,
                arguments: args(&["file with spaces.log"])
            }
        );
    }

    #[test]
    fn tui_requires_a_file() {
        assert!(matches!(
            parse_request(&args(&["tui"]), Platform::Linux, GRAPHICAL),
            Err(LauncherError::Usage(_))
        ));
    }

    #[test]
    fn console_detachment_requires_the_launcher_to_be_the_only_console_process() {
        assert!(should_detach_console(&[42], 42));
        assert!(!should_detach_console(&[], 42));
        assert!(!should_detach_console(&[7], 42));
        assert!(!should_detach_console(&[42, 7], 42));
    }

    #[test]
    fn windows_always_selects_desktop_implicitly() {
        assert_eq!(
            parse_request(
                &args(&["server.log"]),
                Platform::Windows,
                HEADLESS_NON_INTERACTIVE
            )
            .unwrap()
            .variant,
            Variant::Desktop
        );
    }

    #[test]
    fn implicit_desktop_requests_preserve_multiple_file_arguments() {
        let arguments = args(&["first.log", "second.log"]);

        assert_eq!(
            parse_request(&arguments, Platform::Windows, HEADLESS_NON_INTERACTIVE).unwrap(),
            LaunchRequest {
                variant: Variant::Desktop,
                arguments: arguments.clone(),
            }
        );
        assert_eq!(
            parse_request(&arguments, Platform::Linux, GRAPHICAL).unwrap(),
            LaunchRequest {
                variant: Variant::Desktop,
                arguments,
            }
        );
    }

    #[test]
    fn linux_selects_desktop_for_graphical_sessions() {
        assert_eq!(
            parse_request(&args(&["server.log"]), Platform::Linux, GRAPHICAL)
                .unwrap()
                .variant,
            Variant::Desktop
        );
        assert_eq!(
            parse_request(&args(&[]), Platform::Linux, GRAPHICAL)
                .unwrap()
                .variant,
            Variant::Desktop
        );
    }

    #[test]
    fn linux_selects_tui_only_for_a_headless_interactive_file_request() {
        assert_eq!(
            parse_request(&args(&["server.log"]), Platform::Linux, HEADLESS_TERMINAL)
                .unwrap()
                .variant,
            Variant::Tui
        );
    }

    #[test]
    fn linux_reports_actionable_errors_when_no_implicit_variant_is_valid() {
        assert!(matches!(
            parse_request(&args(&[]), Platform::Linux, HEADLESS_TERMINAL),
            Err(LauncherError::MissingDisplay)
        ));
        assert!(matches!(
            parse_request(
                &args(&["server.log"]),
                Platform::Linux,
                HEADLESS_NON_INTERACTIVE
            ),
            Err(LauncherError::NonInteractive)
        ));
    }

    #[test]
    fn sibling_executable_resolution_supports_directories_with_spaces() {
        let directory = tempfile::tempdir()
            .unwrap()
            .keep()
            .join("suite with spaces");
        fs::create_dir_all(&directory).unwrap();
        let executable = directory.join("logmancer-web");
        fs::write(&executable, "").unwrap();

        assert_eq!(
            resolve_sibling_executable(&directory, Variant::Web, Platform::Linux).unwrap(),
            executable
        );
        fs::remove_dir_all(directory.parent().unwrap()).unwrap();
    }

    #[test]
    fn missing_sibling_executable_is_actionable() {
        let directory = tempfile::tempdir().unwrap();
        let error = resolve_sibling_executable(directory.path(), Variant::Desktop, Platform::Linux)
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Reinstall the complete Logmancer suite")
        );
    }

    #[test]
    fn windows_executable_resolution_uses_exe_extension() {
        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("logmancer-desktop.exe");
        fs::write(&executable, "").unwrap();

        assert_eq!(
            resolve_sibling_executable(directory.path(), Variant::Desktop, Platform::Windows)
                .unwrap(),
            executable
        );
    }

    #[cfg(unix)]
    #[test]
    fn launcher_forwards_arguments_environment_and_exit_status() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("logmancer-web");
        let output = directory.path().join("forwarded.txt");
        fs::write(
            &executable,
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$1\"\nprintf '%s\\n' \"$PATH\" >> \"$1\"\nexit 17\n",
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();

        let status = launch(
            LaunchRequest {
                variant: Variant::Web,
                arguments: vec![
                    output.clone().into_os_string(),
                    OsString::from("file with spaces.log"),
                ],
            },
            directory.path(),
            Platform::Linux,
        )
        .unwrap();

        assert_eq!(status.code(), Some(17));
        let forwarded = fs::read_to_string(output).unwrap();
        assert_eq!(forwarded.lines().nth(1), Some("file with spaces.log"));
        assert!(!forwarded.lines().nth(2).unwrap().is_empty());
    }
}
