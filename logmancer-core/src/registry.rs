#[cfg(feature = "native-persistence")]
use crate::recent_files::RecentFilesManager;
use crate::{
    LogReader, SaveResult, VisualRulesEnvelope, VisualRulesError, VisualRulesManager,
    VisualRulesState,
};
use dashmap::DashMap;
use dashmap::mapref::one::RefMut;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

pub trait FileOpenPolicy: Send + Sync {
    fn validate(&self, path: &Path) -> io::Result<PathBuf>;
}

pub struct LogRegistry {
    open_files: Arc<DashMap<Uuid, LogReader>>,
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

    /// Gets a LogReader by UUID
    pub fn get_reader(&self, file_id: &str) -> Option<RefMut<'_, Uuid, LogReader>> {
        let uuid = Uuid::parse_str(file_id).ok()?;
        if self.open_files.contains_key(&uuid) {
            return self.open_files.get_mut(&uuid);
        }
        #[cfg(feature = "native-persistence")]
        if let Some(manager) = &self.recent_files_manager {
            let path = manager.path_for_id(file_id)?;
            let path = self.resolve_path(&path).ok()?;
            self.open_with_id(uuid, &path).ok()?;
            manager
                .record(file_id.to_string(), path.to_string_lossy().into_owned())
                .ok()?;
        }
        self.open_files.get_mut(&uuid)
    }

    fn resolve_path(&self, path: &str) -> io::Result<PathBuf> {
        match &self.file_open_policy {
            Some(policy) => policy.validate(Path::new(path)),
            None => Path::new(path).canonicalize(),
        }
    }

    fn open_with_id(&self, uuid: Uuid, path: &Path) -> io::Result<()> {
        if self.open_files.contains_key(&uuid) {
            return Ok(());
        }
        let reader = LogReader::with_manager(
            path.to_string_lossy().into_owned(),
            self.visual_rules_manager.clone(),
        )?;
        self.open_files.insert(uuid, reader);
        Ok(())
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
        let file_info = registry.get_reader(&file_id).unwrap().file_info().unwrap();

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
