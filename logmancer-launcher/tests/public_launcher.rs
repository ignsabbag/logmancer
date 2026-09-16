#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

static LAUNCHER_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
    let _lock = LAUNCHER_TEST_LOCK.lock().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let launcher = portable_launcher(directory.path());
    web_fixture(directory.path(), "#!/bin/sh\nexit 23\n");

    let status = Command::new(launcher).arg("web").status().unwrap();

    assert_eq!(status.code(), Some(23));
}

#[test]
fn public_launcher_propagates_termination_signals() {
    let _lock = LAUNCHER_TEST_LOCK.lock().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let launcher = portable_launcher(directory.path());
    web_fixture(directory.path(), "#!/bin/sh\nkill -TERM $$\n");

    let status = Command::new(launcher).arg("web").status().unwrap();

    assert_eq!(
        std::os::unix::process::ExitStatusExt::signal(&status),
        Some(libc::SIGTERM)
    );
}

#[test]
fn public_launcher_supplies_leptos_defaults_without_overriding_user_values() {
    let _lock = LAUNCHER_TEST_LOCK.lock().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let suite_directory = directory.path().join("suite with spaces");
    fs::create_dir(&suite_directory).unwrap();
    let launcher = portable_launcher(&suite_directory);
    fs::create_dir(suite_directory.join("site")).unwrap();
    fs::write(suite_directory.join("site/index.html"), "").unwrap();
    fs::create_dir(suite_directory.join("site/pkg")).unwrap();
    web_fixture(
        &suite_directory,
        "#!/bin/sh\nprintf '%s|%s' \"$LEPTOS_OUTPUT_NAME\" \"$LEPTOS_SITE_ROOT\"\n",
    );

    let output = Command::new(&launcher)
        .arg("web")
        .env("LEPTOS_OUTPUT_NAME", "")
        .env("LEPTOS_SITE_ROOT", "")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("logmancer-web|{}", suite_directory.join("site").display())
    );

    let output = Command::new(&launcher)
        .arg("web")
        .env("LEPTOS_OUTPUT_NAME", "custom-output")
        .env("LEPTOS_SITE_ROOT", "/custom/site")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "custom-output|/custom/site"
    );
}

#[test]
fn public_launcher_supplies_leptos_defaults_to_desktop_and_keeps_tui_unchanged() {
    let _lock = LAUNCHER_TEST_LOCK.lock().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let launcher = portable_launcher(directory.path());
    fs::create_dir(directory.path().join("site")).unwrap();
    fs::write(directory.path().join("site/index.html"), "").unwrap();
    fs::create_dir(directory.path().join("site/pkg")).unwrap();
    fs::write(
        directory.path().join("logmancer-desktop"),
        "#!/bin/sh\nprintf '%s|%s' \"$LEPTOS_OUTPUT_NAME\" \"$LEPTOS_SITE_ROOT\"\n",
    )
    .unwrap();
    fs::set_permissions(
        directory.path().join("logmancer-desktop"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    fs::write(
        directory.path().join("logmancer-tui"),
        "#!/bin/sh\nprintf '%s' \"${LEPTOS_OUTPUT_NAME-unset}\"\n",
    )
    .unwrap();
    fs::set_permissions(
        directory.path().join("logmancer-tui"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();

    let desktop = Command::new(&launcher).arg("desktop").output().unwrap();
    assert!(desktop.status.success());
    assert_eq!(
        String::from_utf8(desktop.stdout).unwrap(),
        format!("logmancer-web|{}", directory.path().join("site").display())
    );

    let tui = Command::new(&launcher)
        .args(["tui", "example.log"])
        .env_remove("LEPTOS_OUTPUT_NAME")
        .env_remove("LEPTOS_SITE_ROOT")
        .output()
        .unwrap();
    assert!(tui.status.success());
    assert_eq!(String::from_utf8(tui.stdout).unwrap(), "unset");
}
