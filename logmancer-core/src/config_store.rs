use crate::recent_files::RecentFilesManager;
use crate::{NativeVisualRulesStore, VisualRulesStore};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const VISUAL_RULES_FILE: &str = "visual-rules.json";
const RECENT_FILES_FILE: &str = "recent-files.json";

#[derive(Clone)]
pub struct ConfigStore {
    directory: PathBuf,
}

impl ConfigStore {
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }

    pub fn prepare(&self) -> io::Result<()> {
        std::fs::create_dir_all(&self.directory)
    }

    pub fn visual_rules(&self) -> Arc<dyn VisualRulesStore> {
        Arc::new(NativeVisualRulesStore::new(
            self.directory.join(VISUAL_RULES_FILE),
        ))
    }

    pub fn recent_files(&self) -> io::Result<RecentFilesManager> {
        RecentFilesManager::load(self.directory.join(RECENT_FILES_FILE))
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepares_its_directory_and_owns_visual_rules_file_name() {
        let directory =
            std::env::temp_dir().join(format!("logmancer-config-store-{}", uuid::Uuid::new_v4()));
        let store = ConfigStore::new(directory.clone());

        store.prepare().unwrap();
        store.visual_rules().save_new(b"{}").unwrap();

        assert!(directory.join(VISUAL_RULES_FILE).is_file());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
