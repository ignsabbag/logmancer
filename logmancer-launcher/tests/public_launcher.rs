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

fn ssr_site_fixture(directory: &Path) {
    let package_directory = directory.join("site/pkg");
    fs::create_dir_all(&package_directory).unwrap();
    fs::write(package_directory.join("logmancer-web.css"), "").unwrap();
    fs::write(package_directory.join("logmancer-web.js"), "").unwrap();
    fs::write(package_directory.join("logmancer-web.wasm"), "").unwrap();
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
    ssr_site_fixture(&suite_directory);
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
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Resolved LEPTOS_OUTPUT_NAME from default"));
    assert!(stderr.contains("Resolved LEPTOS_SITE_ROOT from installed resource"));

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
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Resolved LEPTOS_OUTPUT_NAME from environment"));
    assert!(stderr.contains("Resolved LEPTOS_SITE_ROOT from environment"));
}

#[test]
fn public_launcher_preserves_leptos_parameter_sources_for_web_logs() {
    let _lock = LAUNCHER_TEST_LOCK.lock().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let launcher = portable_launcher(directory.path());
    ssr_site_fixture(directory.path());
    web_fixture(
        directory.path(),
        "#!/bin/sh\nprintf '{\"output_name_source\":\"%s\",\"site_root_source\":\"%s\"}\\n' \"${LOGMANCER_LEPTOS_OUTPUT_NAME_SOURCE:-environment}\" \"${LOGMANCER_LEPTOS_SITE_ROOT_SOURCE:-environment}\" >&2\n",
    );

    let output = Command::new(launcher)
        .arg("web")
        .env_remove("LEPTOS_OUTPUT_NAME")
        .env_remove("LEPTOS_SITE_ROOT")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains(
        "{\"output_name_source\":\"default\",\"site_root_source\":\"installed_resource\"}"
    ));
}

#[test]
fn public_launcher_supplies_leptos_defaults_to_desktop_and_keeps_tui_unchanged() {
    let _lock = LAUNCHER_TEST_LOCK.lock().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let launcher = portable_launcher(directory.path());
    ssr_site_fixture(directory.path());
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
