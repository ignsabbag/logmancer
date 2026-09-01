#![cfg(feature = "native-persistence")]

use logmancer_core::{ConfigStore, FileOpenPolicy, LogRegistry};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

struct RejectPolicy;

impl FileOpenPolicy for RejectPolicy {
    fn validate(&self, _path: &Path) -> io::Result<PathBuf> {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "persisted file is not authorized",
        ))
    }
}

struct FailingPolicy;

impl FileOpenPolicy for FailingPolicy {
    fn validate(&self, _path: &Path) -> io::Result<PathBuf> {
        Err(io::Error::other("policy service unavailable"))
    }
}

fn registry(directory: &std::path::Path) -> LogRegistry {
    let store = ConfigStore::new(directory.to_path_buf());
    store.prepare().unwrap();
    LogRegistry::builder().config_store(store).build()
}

#[test]
fn reopening_a_file_reuses_its_persisted_id_after_restart() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("application.log");
    std::fs::write(&file, "INFO ready\n").unwrap();

    let first = registry(&directory.path().join("config"));
    let id = first.open_file(file.to_str().unwrap()).unwrap();
    drop(first);

    let restarted = registry(&directory.path().join("config"));
    assert!(matches!(restarted.with_reader(&id, |_| ()), Ok(Some(()))));
}

#[test]
fn ephemeral_opening_is_not_restored_after_restart() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("upload.log");
    std::fs::write(&file, "INFO uploaded\n").unwrap();

    let first = registry(&directory.path().join("config"));
    let id = first.open_ephemeral_file(file.to_str().unwrap()).unwrap();
    drop(first);

    let restarted = registry(&directory.path().join("config"));
    assert!(matches!(restarted.with_reader(&id, |_| ()), Ok(None)));
}

#[test]
fn persistent_history_keeps_only_the_ten_most_recent_files() {
    let directory = tempfile::tempdir().unwrap();
    let open_registry = registry(&directory.path().join("config"));
    let mut ids = Vec::new();
    for index in 0..11 {
        let file = directory.path().join(format!("{index}.log"));
        std::fs::write(&file, "INFO ready\n").unwrap();
        ids.push(open_registry.open_file(file.to_str().unwrap()).unwrap());
    }
    drop(open_registry);

    let restarted = registry(&directory.path().join("config"));
    assert!(matches!(restarted.with_reader(&ids[0], |_| ()), Ok(None)));
    assert!(matches!(
        restarted.with_reader(&ids[10], |_| ()),
        Ok(Some(()))
    ));
}

#[test]
fn concurrent_registries_merge_recent_file_updates() {
    let directory = tempfile::tempdir().unwrap();
    let first_file = directory.path().join("first.log");
    let second_file = directory.path().join("second.log");
    std::fs::write(&first_file, "INFO first\n").unwrap();
    std::fs::write(&second_file, "INFO second\n").unwrap();

    let config = directory.path().join("config");
    let first_registry = registry(&config);
    let second_registry = registry(&config);
    let first_id = first_registry
        .open_file(first_file.to_str().unwrap())
        .unwrap();
    let second_id = second_registry
        .open_file(second_file.to_str().unwrap())
        .unwrap();
    drop((first_registry, second_registry));

    let restarted = registry(&config);
    assert!(matches!(
        restarted.with_reader(&first_id, |_| ()),
        Ok(Some(()))
    ));
    assert!(matches!(
        restarted.with_reader(&second_id, |_| ()),
        Ok(Some(()))
    ));
}

#[test]
fn deleted_persisted_file_returns_not_found_restoration_error() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("deleted.log");
    std::fs::write(&file, "INFO ready\n").unwrap();
    let config = directory.path().join("config");
    let first = registry(&config);
    let id = first.open_file(file.to_str().unwrap()).unwrap();
    drop(first);
    std::fs::remove_file(file).unwrap();

    let error = registry(&config).with_reader(&id, |_| ()).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert!(error.to_string().contains("restore persisted reader"));
}

#[test]
fn rejected_persisted_file_returns_permission_denied_restoration_error() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("restricted.log");
    std::fs::write(&file, "INFO ready\n").unwrap();
    let config = directory.path().join("config");
    let first = registry(&config);
    let id = first.open_file(file.to_str().unwrap()).unwrap();
    drop(first);
    let store = ConfigStore::new(config);
    store.prepare().unwrap();
    let restarted = LogRegistry::builder()
        .config_store(store)
        .file_open_policy(Arc::new(RejectPolicy))
        .build();

    let error = restarted.with_reader(&id, |_| ()).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    assert!(error.to_string().contains("restore persisted reader"));
}

#[test]
fn other_persisted_restoration_failures_preserve_error_kind_and_context() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("application.log");
    std::fs::write(&file, "INFO ready\n").unwrap();
    let config = directory.path().join("config");
    let first = registry(&config);
    let id = first.open_file(file.to_str().unwrap()).unwrap();
    drop(first);
    let store = ConfigStore::new(config);
    store.prepare().unwrap();
    let restarted = LogRegistry::builder()
        .config_store(store)
        .file_open_policy(Arc::new(FailingPolicy))
        .build();

    let error = restarted.with_reader(&id, |_| ()).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::Other);
    assert!(error.to_string().contains("restore persisted reader"));
    assert!(error.to_string().contains("policy service unavailable"));
}

#[test]
fn invalid_and_unknown_ids_return_none() {
    let directory = tempfile::tempdir().unwrap();
    let registry = registry(directory.path());

    assert!(matches!(
        registry.with_reader("not-a-uuid", |_| ()),
        Ok(None)
    ));
    assert!(matches!(
        registry.with_reader("00000000-0000-0000-0000-000000000000", |_| ()),
        Ok(None)
    ));
}

#[test]
fn corrupt_history_is_backed_up_and_reinitialized() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("recent-files.json"), "not json").unwrap();
    let file = directory.path().join("application.log");
    std::fs::write(&file, "INFO ready\n").unwrap();

    let registry = registry(&config);
    let id = registry.open_file(file.to_str().unwrap()).unwrap();
    drop(registry);

    let history = std::fs::read_to_string(config.join("recent-files.json")).unwrap();
    assert!(history.contains(&id));
    assert!(std::fs::read_dir(&config).unwrap().flatten().any(|entry| {
        entry
            .file_name()
            .to_string_lossy()
            .starts_with("recent-files.")
            && entry
                .file_name()
                .to_string_lossy()
                .ends_with(".corrupt.json")
    }));
}
