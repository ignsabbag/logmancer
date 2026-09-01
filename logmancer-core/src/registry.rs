#[cfg(feature = "native-persistence")]
use crate::recent_files::RecentFilesManager;
use crate::{
    LogReader, SaveResult, VisualRulesEnvelope, VisualRulesError, VisualRulesManager,
    VisualRulesState,
};
use dashmap::DashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Instant;
use uuid::Uuid;

pub trait FileOpenPolicy: Send + Sync {
    fn validate(&self, path: &Path) -> io::Result<PathBuf>;
}

struct ReaderEntry {
    reader: Mutex<Option<LogReader>>,
    state: Arc<(Mutex<ReaderState>, Condvar)>,
}

struct ReaderState {
    active_leases: usize,
    closing: bool,
    last_access: Instant,
}

struct ReaderLease {
    state: Arc<(Mutex<ReaderState>, Condvar)>,
}

struct RegistryLifecycle {
    state: Mutex<RegistryLifecycleState>,
    changed: Condvar,
}

struct RegistryLifecycleState {
    #[cfg(test)]
    opening_waiters: usize,
    #[cfg(test)]
    open_candidate_barrier: Option<Arc<std::sync::Barrier>>,
}

impl ReaderEntry {
    fn new(reader: LogReader) -> Self {
        Self {
            reader: Mutex::new(Some(reader)),
            state: Arc::new((
                Mutex::new(ReaderState {
                    active_leases: 0,
                    closing: false,
                    last_access: Instant::now(),
                }),
                Condvar::new(),
            )),
        }
    }

    fn acquire(&self) -> Option<ReaderLease> {
        let mut state = self.state.0.lock().unwrap();
        if state.closing {
            return None;
        }
        state.active_leases += 1;
        state.last_access = Instant::now();
        Some(ReaderLease {
            state: self.state.clone(),
        })
    }

    fn begin_closing(&self) {
        let (state_lock, leases_drained) = &*self.state;
        let mut state = state_lock.lock().unwrap();
        state.closing = true;
        leases_drained.notify_all();
    }

    fn is_closing(&self) -> bool {
        self.state.0.lock().unwrap().closing
    }

    fn wait_for_leases_and_take(&self) -> Option<LogReader> {
        let (state_lock, leases_drained) = &*self.state;
        let mut state = state_lock.lock().unwrap();
        while state.active_leases > 0 {
            state = leases_drained.wait(state).unwrap();
        }
        drop(state);

        self.reader
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
    }

    #[cfg(test)]
    fn active_leases(&self) -> usize {
        self.state.0.lock().unwrap().active_leases
    }

    #[cfg(test)]
    fn wait_for_closing(&self) {
        let (state_lock, closing_changed) = &*self.state;
        let mut state = state_lock.lock().unwrap();
        while !state.closing {
            state = closing_changed.wait(state).unwrap();
        }
    }

    #[cfg(test)]
    fn resource_weak(
        &self,
    ) -> std::sync::Weak<std::sync::RwLock<crate::models::log_file::LogFile>> {
        self.reader
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .resource_weak()
    }
}

impl Drop for ReaderLease {
    fn drop(&mut self) {
        let (state_lock, leases_drained) = &*self.state;
        let mut state = state_lock.lock().unwrap();
        state.active_leases -= 1;
        if state.active_leases == 0 {
            leases_drained.notify_all();
        }
    }
}

pub struct LogRegistry {
    open_files: Arc<DashMap<Uuid, Arc<ReaderEntry>>>,
    lifecycle: Arc<RegistryLifecycle>,
    visual_rules_manager: Arc<VisualRulesManager>,
    file_open_policy: Option<Arc<dyn FileOpenPolicy>>,
    #[cfg(feature = "native-persistence")]
    recent_files_manager: Option<Arc<RecentFilesManager>>,
}

pub struct LogRegistryBuilder {
    #[cfg(feature = "native-persistence")]
    config_store: Option<crate::ConfigStore>,
    file_open_policy: Option<Arc<dyn FileOpenPolicy>>,
}

impl LogRegistryBuilder {
    #[cfg(feature = "native-persistence")]
    pub fn config_store(mut self, config_store: crate::ConfigStore) -> Self {
        self.config_store = Some(config_store);
        self
    }

    pub fn file_open_policy(mut self, file_open_policy: Arc<dyn FileOpenPolicy>) -> Self {
        self.file_open_policy = Some(file_open_policy);
        self
    }

    pub fn build(self) -> LogRegistry {
        #[cfg(feature = "native-persistence")]
        let visual_rules_manager = self
            .config_store
            .as_ref()
            .map(|config_store| VisualRulesManager::with_store(config_store.visual_rules()))
            .unwrap_or_else(VisualRulesManager::in_memory);
        #[cfg(not(feature = "native-persistence"))]
        let visual_rules_manager = VisualRulesManager::in_memory();

        #[cfg(feature = "native-persistence")]
        let recent_files_manager = self
            .config_store
            .as_ref()
            .and_then(|store| store.recent_files().ok())
            .map(Arc::new);
        LogRegistry {
            open_files: Arc::new(DashMap::new()),
            lifecycle: Arc::new(RegistryLifecycle {
                state: Mutex::new(RegistryLifecycleState {
                    #[cfg(test)]
                    opening_waiters: 0,
                    #[cfg(test)]
                    open_candidate_barrier: None,
                }),
                changed: Condvar::new(),
            }),
            visual_rules_manager,
            file_open_policy: self.file_open_policy,
            #[cfg(feature = "native-persistence")]
            recent_files_manager,
        }
    }
}

impl LogRegistry {
    pub fn builder() -> LogRegistryBuilder {
        LogRegistryBuilder {
            #[cfg(feature = "native-persistence")]
            config_store: None,
            file_open_policy: None,
        }
    }

    pub fn new() -> Self {
        Self::builder().build()
    }

    /// Opens a new file and register with a UUID
    pub fn open_file(&self, path: &str) -> io::Result<String> {
        let path = self.resolve_path(path)?;
        #[cfg(feature = "native-persistence")]
        if let Some(manager) = &self.recent_files_manager
            && let Some(id) = manager.id_for_path(&path.to_string_lossy())
        {
            let uuid = Uuid::parse_str(&id).map_err(io::Error::other)?;
            self.open_with_id(uuid, &path)?;
            manager.record(id.clone(), path.to_string_lossy().into_owned())?;
            return Ok(id);
        }
        let uuid = Uuid::new_v4();
        self.open_with_id(uuid, &path)?;
        #[cfg(feature = "native-persistence")]
        if let Some(manager) = &self.recent_files_manager {
            manager.record(uuid.to_string(), path.to_string_lossy().into_owned())?;
        }
        Ok(uuid.to_string())
    }

    pub fn open_ephemeral_file(&self, path: &str) -> io::Result<String> {
        let uuid = Uuid::new_v4();
        let path = self.resolve_path(path)?;
        self.open_with_id(uuid, &path)?;
        Ok(uuid.to_string())
    }

    pub fn with_reader<T>(
        &self,
        file_id: &str,
        operation: impl FnOnce(&mut LogReader) -> T,
    ) -> io::Result<Option<T>> {
        loop {
            let Some(entry) = self.reader_entry(file_id)? else {
                return Ok(None);
            };
            if let Some(_lease) = entry.acquire() {
                let mut reader = entry.reader.lock().unwrap();
                let Some(reader) = reader.as_mut() else {
                    return Ok(None);
                };
                return Ok(Some(operation(reader)));
            }
        }
    }

    pub fn remove_reader(&self, file_id: &str) -> Option<()> {
        let uuid = Uuid::parse_str(file_id).ok()?;
        let lifecycle = self.lifecycle.state.lock().unwrap();
        let entry = self.open_files.get(&uuid).map(|entry| entry.clone())?;
        entry.begin_closing();
        drop(lifecycle);

        let reader = entry.wait_for_leases_and_take()?;
        drop(reader);

        let lifecycle = self.lifecycle.state.lock().unwrap();
        self.open_files
            .remove_if(&uuid, |_, current| Arc::ptr_eq(current, &entry));
        self.lifecycle.changed.notify_all();
        drop(lifecycle);
        Some(())
    }

    fn reader_entry(&self, file_id: &str) -> io::Result<Option<Arc<ReaderEntry>>> {
        let Ok(uuid) = Uuid::parse_str(file_id) else {
            return Ok(None);
        };
        loop {
            if let Some(entry) = self.usable_entry(uuid) {
                return Ok(Some(entry));
            }
            #[cfg(feature = "native-persistence")]
            if let Some(manager) = &self.recent_files_manager {
                let Some(path) = manager.path_for_id(file_id) else {
                    return Ok(None);
                };
                let path = self
                    .resolve_path(&path)
                    .map_err(|error| Self::restoration_error("resolve its path", error))?;
                self.open_with_id(uuid, &path)
                    .map_err(|error| Self::restoration_error("open its file", error))?;
                manager
                    .record(file_id.to_string(), path.to_string_lossy().into_owned())
                    .map_err(|error| Self::restoration_error("update recent files", error))?;
                continue;
            }
            return Ok(None);
        }
    }

    #[cfg(test)]
    fn active_leases(&self, file_id: &str) -> Option<usize> {
        let uuid = Uuid::parse_str(file_id).ok()?;
        Some(self.open_files.get(&uuid)?.active_leases())
    }

    #[cfg(test)]
    fn wait_for_closing(&self, file_id: &str) -> Option<()> {
        let uuid = Uuid::parse_str(file_id).ok()?;
        self.open_files.get(&uuid)?.wait_for_closing();
        Some(())
    }

    #[cfg(test)]
    fn resource_weak(
        &self,
        file_id: &str,
    ) -> Option<std::sync::Weak<std::sync::RwLock<crate::models::log_file::LogFile>>> {
        let uuid = Uuid::parse_str(file_id).ok()?;
        Some(self.open_files.get(&uuid)?.resource_weak())
    }

    #[cfg(test)]
    fn wait_for_open_waiter(&self) {
        let mut lifecycle = self.lifecycle.state.lock().unwrap();
        while lifecycle.opening_waiters == 0 {
            lifecycle = self.lifecycle.changed.wait(lifecycle).unwrap();
        }
    }

    #[cfg(test)]
    fn pause_open_candidates(&self, barrier: Arc<std::sync::Barrier>) {
        self.lifecycle.state.lock().unwrap().open_candidate_barrier = Some(barrier);
    }

    #[cfg(test)]
    fn reader_entry_for_test(&self, uuid: Uuid) -> Option<Arc<ReaderEntry>> {
        self.open_files.get(&uuid).map(|entry| entry.clone())
    }

    #[cfg(test)]
    fn open_file_count(&self) -> usize {
        self.open_files.len()
    }

    #[cfg(test)]
    fn replace_reader_for_test(&self, file_id: &str, path: &Path) -> io::Result<Arc<ReaderEntry>> {
        let uuid = Uuid::parse_str(file_id).map_err(io::Error::other)?;
        let reader = LogReader::with_manager(
            path.to_string_lossy().into_owned(),
            self.visual_rules_manager.clone(),
        )?;
        let entry = Arc::new(ReaderEntry::new(reader));
        self.open_files.insert(uuid, entry.clone());
        Ok(entry)
    }

    fn resolve_path(&self, path: &str) -> io::Result<PathBuf> {
        match &self.file_open_policy {
            Some(policy) => policy.validate(Path::new(path)),
            None => Path::new(path).canonicalize(),
        }
    }

    fn restoration_error(action: &str, error: io::Error) -> io::Error {
        io::Error::new(
            error.kind(),
            format!("Could not restore persisted reader while attempting to {action}: {error}"),
        )
    }

    fn open_with_id(&self, uuid: Uuid, path: &Path) -> io::Result<()> {
        loop {
            if self.usable_entry(uuid).is_some() {
                return Ok(());
            }

            let reader = LogReader::with_manager(
                path.to_string_lossy().into_owned(),
                self.visual_rules_manager.clone(),
            )?;
            let candidate = Arc::new(ReaderEntry::new(reader));
            #[cfg(test)]
            let open_candidate_barrier = self
                .lifecycle
                .state
                .lock()
                .unwrap()
                .open_candidate_barrier
                .clone();
            #[cfg(test)]
            if let Some(barrier) = open_candidate_barrier {
                barrier.wait();
            }

            let lifecycle = self.lifecycle.state.lock().unwrap();
            if self.open_files.contains_key(&uuid) {
                drop(lifecycle);
                drop(candidate);
                continue;
            }
            self.open_files.insert(uuid, candidate);
            drop(lifecycle);
            return Ok(());
        }
    }

    fn usable_entry(&self, uuid: Uuid) -> Option<Arc<ReaderEntry>> {
        let mut lifecycle = self.lifecycle.state.lock().unwrap();
        loop {
            let entry = self.open_files.get(&uuid).map(|entry| entry.clone())?;
            if !entry.is_closing() {
                return Some(entry);
            }
            #[cfg(test)]
            {
                lifecycle.opening_waiters += 1;
                self.lifecycle.changed.notify_all();
            }
            lifecycle = self.lifecycle.changed.wait(lifecycle).unwrap();
            #[cfg(test)]
            {
                lifecycle.opening_waiters -= 1;
            }
        }
    }

    pub fn visual_rules_state(&self) -> VisualRulesState {
        self.visual_rules_manager.state()
    }

    pub fn apply_visual_rules_memory(
        &self,
        envelope: VisualRulesEnvelope,
    ) -> Result<SaveResult, VisualRulesError> {
        self.visual_rules_manager.apply_memory(envelope)
    }

    #[cfg(feature = "native-persistence")]
    pub fn reload_visual_rules(&self) -> Result<VisualRulesState, VisualRulesError> {
        self.visual_rules_manager.load()
    }

    #[cfg(feature = "native-persistence")]
    pub fn upsert_visual_rules(
        &self,
        base_revision: u64,
        envelope: VisualRulesEnvelope,
    ) -> Result<SaveResult, VisualRulesError> {
        self.visual_rules_manager.upsert(base_revision, envelope)
    }
}

impl Default for LogRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Barrier, mpsc::TryRecvError};

    struct RedirectPolicy {
        path: PathBuf,
    }

    impl FileOpenPolicy for RedirectPolicy {
        fn validate(&self, _path: &Path) -> io::Result<PathBuf> {
            Ok(self.path.clone())
        }
    }

    struct RejectPolicy;

    impl FileOpenPolicy for RejectPolicy {
        fn validate(&self, _path: &Path) -> io::Result<PathBuf> {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "rejected"))
        }
    }

    #[test]
    fn policy_path_is_used_to_open_the_reader() {
        let directory =
            std::env::temp_dir().join(format!("logmancer-registry-policy-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let allowed_path = directory.join("allowed.log");
        std::fs::write(&allowed_path, "ready\n").unwrap();
        let registry = LogRegistry::builder()
            .file_open_policy(Arc::new(RedirectPolicy {
                path: allowed_path.clone(),
            }))
            .build();

        let file_id = registry.open_file("untrusted.log").unwrap();
        let file_info = registry
            .with_reader(&file_id, |reader| reader.file_info())
            .unwrap()
            .unwrap()
            .unwrap();

        assert_eq!(file_info.path, allowed_path.to_string_lossy());
        drop(registry);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn rejected_policy_prevents_reader_creation() {
        let registry = LogRegistry::builder()
            .file_open_policy(Arc::new(RejectPolicy))
            .build();

        let error = registry.open_file("blocked.log").unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn leased_access_releases_the_lease_after_an_operation_error() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("application.log");
        std::fs::write(&path, "ready\n").unwrap();
        let registry = LogRegistry::new();
        let file_id = registry.open_file(path.to_str().unwrap()).unwrap();

        let result = registry.with_reader(&file_id, |_| {
            Err::<(), _>(io::Error::other("operation failed"))
        });

        assert!(
            result
                .expect("registry access succeeds")
                .expect("reader is available")
                .is_err()
        );
        assert_eq!(registry.active_leases(&file_id), Some(0));
    }

    #[test]
    fn access_during_removal_waits_until_the_reader_is_removed() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("application.log");
        std::fs::write(&path, "ready\n").unwrap();
        let registry = Arc::new(LogRegistry::new());
        let file_id = registry.open_file(path.to_str().unwrap()).unwrap();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let active_registry = registry.clone();
        let active_file_id = file_id.clone();

        let active_operation = std::thread::spawn(move || {
            active_registry.with_reader(&active_file_id, |_| {
                started_tx.send(()).unwrap();
                release_rx.recv().unwrap();
            })
        });
        started_rx.recv().unwrap();

        let removing_registry = registry.clone();
        let removing_file_id = file_id.clone();
        let removal =
            std::thread::spawn(move || removing_registry.remove_reader(&removing_file_id));
        registry.wait_for_closing(&file_id).unwrap();

        let accessing_registry = registry.clone();
        let accessing_file_id = file_id.clone();
        let (completed_tx, completed_rx) = std::sync::mpsc::channel();
        let access = std::thread::spawn(move || {
            completed_tx
                .send(accessing_registry.with_reader(&accessing_file_id, |_| ()))
                .unwrap();
        });
        registry.wait_for_open_waiter();

        assert!(matches!(completed_rx.try_recv(), Err(TryRecvError::Empty)));
        release_tx.send(()).unwrap();
        assert!(active_operation.join().unwrap().unwrap().is_some());
        assert!(removal.join().unwrap().is_some());
        assert!(completed_rx.recv().unwrap().unwrap().is_none());
        access.join().unwrap();
    }

    #[test]
    fn removing_an_idle_reader_releases_its_resources() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("application.log");
        std::fs::write(&path, "ready\n").unwrap();
        let registry = LogRegistry::new();
        let file_id = registry.open_file(path.to_str().unwrap()).unwrap();
        let resources = registry.resource_weak(&file_id).unwrap();

        assert!(registry.remove_reader(&file_id).is_some());

        assert!(resources.upgrade().is_none());
        assert!(matches!(registry.with_reader(&file_id, |_| ()), Ok(None)));
    }

    #[test]
    fn removing_a_reader_waits_for_an_active_lease() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("application.log");
        std::fs::write(&path, "ready\n").unwrap();
        let registry = Arc::new(LogRegistry::new());
        let file_id = registry.open_file(path.to_str().unwrap()).unwrap();
        let resources = registry.resource_weak(&file_id).unwrap();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let active_registry = registry.clone();
        let active_file_id = file_id.clone();

        let active_operation = std::thread::spawn(move || {
            active_registry.with_reader(&active_file_id, |_| {
                started_tx.send(()).unwrap();
                release_rx.recv().unwrap();
            })
        });
        started_rx.recv().unwrap();

        let removing_registry = registry.clone();
        let removing_file_id = file_id.clone();
        let (removed_tx, removed_rx) = std::sync::mpsc::channel();
        let removal = std::thread::spawn(move || {
            removed_tx
                .send(removing_registry.remove_reader(&removing_file_id))
                .unwrap();
        });
        registry.wait_for_closing(&file_id).unwrap();

        assert_eq!(removed_rx.try_recv(), Err(TryRecvError::Empty));
        release_tx.send(()).unwrap();
        assert!(matches!(active_operation.join().unwrap(), Ok(Some(()))));
        assert!(removed_rx.recv().unwrap().is_some());
        removal.join().unwrap();
        assert!(resources.upgrade().is_none());
    }

    #[test]
    fn reopening_during_removal_waits_for_a_usable_replacement() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("application.log");
        std::fs::write(&path, "ready\n").unwrap();
        let registry = Arc::new(LogRegistry::new());
        let file_id = registry.open_file(path.to_str().unwrap()).unwrap();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let active_registry = registry.clone();
        let active_file_id = file_id.clone();
        let active_operation = std::thread::spawn(move || {
            active_registry.with_reader(&active_file_id, |_| {
                started_tx.send(()).unwrap();
                release_rx.recv().unwrap();
            })
        });
        started_rx.recv().unwrap();

        let removing_registry = registry.clone();
        let removing_file_id = file_id.clone();
        let removal =
            std::thread::spawn(move || removing_registry.remove_reader(&removing_file_id));
        registry.wait_for_closing(&file_id).unwrap();

        let reopening_registry = registry.clone();
        let reopening_path = path.clone();
        let reopening_file_id = file_id.clone();
        let (opened_tx, opened_rx) = std::sync::mpsc::channel();
        let reopening = std::thread::spawn(move || {
            opened_tx.send(reopening_registry.open_with_id(
                Uuid::parse_str(&reopening_file_id).unwrap(),
                &reopening_path,
            ))
        });
        registry.wait_for_open_waiter();

        assert!(matches!(opened_rx.try_recv(), Err(TryRecvError::Empty)));
        release_tx.send(()).unwrap();
        assert!(matches!(active_operation.join().unwrap(), Ok(Some(()))));
        assert!(removal.join().unwrap().is_some());
        assert!(opened_rx.recv().unwrap().is_ok());
        assert!(reopening.join().unwrap().is_ok());
        assert!(matches!(
            registry.with_reader(&file_id, |_| ()),
            Ok(Some(()))
        ));
    }

    #[cfg(feature = "native-persistence")]
    #[test]
    fn persisted_access_during_removal_waits_for_restoration() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("application.log");
        std::fs::write(&path, "ready\n").unwrap();
        let store = crate::ConfigStore::new(directory.path().join("config"));
        store.prepare().unwrap();
        let registry = Arc::new(LogRegistry::builder().config_store(store).build());
        let file_id = registry.open_file(path.to_str().unwrap()).unwrap();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let active_registry = registry.clone();
        let active_file_id = file_id.clone();
        let active_operation = std::thread::spawn(move || {
            active_registry.with_reader(&active_file_id, |_| {
                started_tx.send(()).unwrap();
                release_rx.recv().unwrap();
            })
        });
        started_rx.recv().unwrap();

        let removing_registry = registry.clone();
        let removing_file_id = file_id.clone();
        let removal =
            std::thread::spawn(move || removing_registry.remove_reader(&removing_file_id));
        registry.wait_for_closing(&file_id).unwrap();

        let restoring_registry = registry.clone();
        let restoring_file_id = file_id.clone();
        let (completed_tx, completed_rx) = std::sync::mpsc::channel();
        let restoration = std::thread::spawn(move || {
            completed_tx
                .send(restoring_registry.with_reader(&restoring_file_id, |_| "restored"))
                .unwrap()
        });
        registry.wait_for_open_waiter();

        assert!(matches!(completed_rx.try_recv(), Err(TryRecvError::Empty)));
        release_tx.send(()).unwrap();
        assert!(active_operation.join().unwrap().unwrap().is_some());
        assert!(removal.join().unwrap().is_some());
        assert_eq!(completed_rx.recv().unwrap().unwrap(), Some("restored"));
        restoration.join().unwrap();
    }

    #[test]
    fn concurrent_opens_keep_the_installed_reader_instance() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("application.log");
        std::fs::write(&path, "ready\n").unwrap();
        let registry = Arc::new(LogRegistry::new());
        let uuid = Uuid::new_v4();
        let barrier = Arc::new(Barrier::new(2));
        registry.pause_open_candidates(barrier);

        let first_registry = registry.clone();
        let first_path = path.clone();
        let first = std::thread::spawn(move || first_registry.open_with_id(uuid, &first_path));
        let second_registry = registry.clone();
        let second_path = path.clone();
        let second = std::thread::spawn(move || second_registry.open_with_id(uuid, &second_path));

        assert!(first.join().unwrap().is_ok());
        assert!(second.join().unwrap().is_ok());
        let installed = registry.reader_entry_for_test(uuid).unwrap();

        registry.open_with_id(uuid, &path).unwrap();

        assert_eq!(registry.open_file_count(), 1);
        assert!(Arc::ptr_eq(
            &installed,
            &registry.reader_entry_for_test(uuid).unwrap()
        ));
    }

    #[test]
    fn removal_keeps_a_replacement_installed_while_the_original_closes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("application.log");
        std::fs::write(&path, "ready\n").unwrap();
        let registry = Arc::new(LogRegistry::new());
        let file_id = registry.open_file(path.to_str().unwrap()).unwrap();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let active_registry = registry.clone();
        let active_file_id = file_id.clone();
        let active_operation = std::thread::spawn(move || {
            active_registry.with_reader(&active_file_id, |_| {
                started_tx.send(()).unwrap();
                release_rx.recv().unwrap();
            })
        });
        started_rx.recv().unwrap();

        let removing_registry = registry.clone();
        let removing_file_id = file_id.clone();
        let removal =
            std::thread::spawn(move || removing_registry.remove_reader(&removing_file_id));
        registry.wait_for_closing(&file_id).unwrap();
        let replacement = registry.replace_reader_for_test(&file_id, &path).unwrap();

        release_tx.send(()).unwrap();
        assert!(matches!(active_operation.join().unwrap(), Ok(Some(()))));
        assert!(removal.join().unwrap().is_some());
        assert!(Arc::ptr_eq(
            &replacement,
            &registry
                .reader_entry_for_test(Uuid::parse_str(&file_id).unwrap())
                .unwrap()
        ));
        assert!(matches!(
            registry.with_reader(&file_id, |_| ()),
            Ok(Some(()))
        ));
    }

    #[cfg(feature = "native-persistence")]
    #[test]
    fn config_store_creates_the_registry_visual_rules_manager() {
        let directory =
            std::env::temp_dir().join(format!("logmancer-registry-config-{}", Uuid::new_v4()));
        let store = crate::ConfigStore::new(directory.join("config"));
        store.prepare().unwrap();
        let registry = LogRegistry::builder().config_store(store).build();

        registry
            .upsert_visual_rules(0, VisualRulesEnvelope::new(Vec::new()))
            .unwrap();

        assert!(directory.join("config/visual-rules.json").is_file());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
