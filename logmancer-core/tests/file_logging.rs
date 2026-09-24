#![cfg(feature = "native-logging")]

use logmancer_core::{init_file_logging, init_file_logging_with_name};
use std::process::Command;

const CHILD_TEST_ENV: &str = "LOGMANCER_FILE_LOGGING_CHILD_TEST";

fn assert_dated_log_exists(directory: &std::path::Path, stem: &str, extension: Option<&str>) {
    let names: Vec<_> = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    let suffix = extension.map_or(String::new(), |extension| format!(".{extension}"));
    assert!(
        names.iter().any(|name| {
            let Some(date) = name
                .strip_prefix(&format!("{stem}."))
                .and_then(|name| name.strip_suffix(&suffix))
            else {
                return false;
            };
            let bytes = date.as_bytes();
            bytes.len() == 10
                && bytes.iter().enumerate().all(|(index, byte)| {
                    if index == 4 || index == 7 {
                        *byte == b'-'
                    } else {
                        byte.is_ascii_digit()
                    }
                })
        }),
        "expected {stem}.YYYY-MM-DD{suffix} in {names:?}"
    );
}

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
    let legacy_file = directory.path().join("custom.log.2026-09-22");

    std::fs::write(&unrelated_file, "do not remove").unwrap();
    std::fs::write(&legacy_file, "old format").unwrap();

    init_file_logging_with_name(directory.path(), "custom.log", false).unwrap();

    assert!(unrelated_file.exists());
    assert!(legacy_file.exists());
    assert_dated_log_exists(directory.path(), "custom", Some("log"));
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

    assert_dated_log_exists(directory.path(), "logmancer-desktop", Some("log"));
    assert!(!directory.path().join("logs").exists());
}

#[test]
fn custom_extension_is_preserved_after_the_date() {
    if std::env::var_os(CHILD_TEST_ENV).is_none() {
        run_in_child("custom_extension_is_preserved_after_the_date");
        return;
    }

    let directory = tempfile::tempdir().unwrap();
    init_file_logging_with_name(directory.path(), "custom.txt", false).unwrap();
    assert_dated_log_exists(directory.path(), "custom", Some("txt"));
}

#[test]
fn extensionless_name_ends_with_the_date() {
    if std::env::var_os(CHILD_TEST_ENV).is_none() {
        run_in_child("extensionless_name_ends_with_the_date");
        return;
    }

    let directory = tempfile::tempdir().unwrap();
    init_file_logging_with_name(directory.path(), "custom", false).unwrap();
    assert_dated_log_exists(directory.path(), "custom", None);
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
