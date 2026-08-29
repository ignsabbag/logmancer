use logmancer_core::{ConfigStore, LogRegistry};

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
    assert!(restarted.get_reader(&id).is_some());
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
    assert!(restarted.get_reader(&id).is_none());
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
    assert!(restarted.get_reader(&ids[0]).is_none());
    assert!(restarted.get_reader(&ids[10]).is_some());
}
