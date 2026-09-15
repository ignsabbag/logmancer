#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

fn portable_launcher(directory: &Path) -> std::path::PathBuf {
    let launcher = directory.join("logmancer");
    fs::copy(env!("CARGO_BIN_EXE_logmancer"), &launcher).unwrap();
    fs::set_permissions(&launcher, fs::Permissions::from_mode(0o755)).unwrap();
    launcher
}

fn web_fixture(directory: &Path, script: &str) {
    let executable = directory.join("logmancer-web");
    fs::write(&executable, script).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn public_launcher_propagates_nonzero_exit_codes() {
    let directory = tempfile::tempdir().unwrap();
    let launcher = portable_launcher(directory.path());
    web_fixture(directory.path(), "#!/bin/sh\nexit 23\n");

    let status = Command::new(launcher).arg("web").status().unwrap();

    assert_eq!(status.code(), Some(23));
}

#[test]
fn public_launcher_propagates_termination_signals() {
    let directory = tempfile::tempdir().unwrap();
    let launcher = portable_launcher(directory.path());
    web_fixture(directory.path(), "#!/bin/sh\nkill -TERM $$\n");

    let status = Command::new(launcher).arg("web").status().unwrap();

    assert_eq!(
        std::os::unix::process::ExitStatusExt::signal(&status),
        Some(libc::SIGTERM)
    );
}
