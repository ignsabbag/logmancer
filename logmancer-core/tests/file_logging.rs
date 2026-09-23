#![cfg(feature = "native-logging")]

use logmancer_core::{init_file_logging, init_file_logging_with_name};
use std::process::Command;

const CHILD_TEST_ENV: &str = "LOGMANCER_FILE_LOGGING_CHILD_TEST";

fn run_in_child(test_name: &str) {
    if std::env::var_os(CHILD_TEST_ENV).is_some() {
        return;
    }

    let status = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg(test_name)
        .arg("--nocapture")
        .env(CHILD_TEST_ENV, "1")
        .status()
        .unwrap();

    assert!(status.success());
}

#[test]
fn custom_log_files_are_written_directly_in_the_requested_directory() {
    if std::env::var_os(CHILD_TEST_ENV).is_none() {
        run_in_child("custom_log_files_are_written_directly_in_the_requested_directory");
        return;
    }

    let directory = tempfile::tempdir().unwrap();
    let unrelated_file = directory.path().join("custom.log.unrelated");

    std::fs::write(&unrelated_file, "do not remove").unwrap();

    init_file_logging_with_name(directory.path(), "custom.log", false).unwrap();

    assert!(unrelated_file.exists());
    assert!(std::fs::read_dir(directory.path()).unwrap().any(|entry| {
        let path = entry.unwrap().path();
        path != unrelated_file
            && path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("custom.log.")
    }));
    assert!(!directory.path().join("logmancer-logs").exists());
}

#[test]
fn variant_log_files_are_written_directly_in_the_requested_directory() {
    if std::env::var_os(CHILD_TEST_ENV).is_none() {
        run_in_child("variant_log_files_are_written_directly_in_the_requested_directory");
        return;
    }

    let directory = tempfile::tempdir().unwrap();
    init_file_logging(directory.path(), "desktop", false).unwrap();

    assert!(std::fs::read_dir(directory.path()).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("logmancer-desktop.log.")
    }));
    assert!(!directory.path().join("logs").exists());
}

#[test]
fn existing_global_subscriber_does_not_block_file_logging_initialization() {
    if std::env::var_os(CHILD_TEST_ENV).is_none() {
        run_in_child("existing_global_subscriber_does_not_block_file_logging_initialization");
        return;
    }

    tracing::subscriber::set_global_default(tracing::subscriber::NoSubscriber::default()).unwrap();
    let directory = tempfile::tempdir().unwrap();

    init_file_logging_with_name(directory.path(), "custom.log", false).unwrap();
}

#[test]
fn invalid_log_directory_remains_an_error_when_a_global_subscriber_exists() {
    if std::env::var_os(CHILD_TEST_ENV).is_none() {
        run_in_child("invalid_log_directory_remains_an_error_when_a_global_subscriber_exists");
        return;
    }

    tracing::subscriber::set_global_default(tracing::subscriber::NoSubscriber::default()).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let invalid_directory = directory.path().join("not-a-directory");
    std::fs::write(&invalid_directory, "blocked").unwrap();

    assert!(init_file_logging_with_name(&invalid_directory, "custom.log", false).is_err());
}
