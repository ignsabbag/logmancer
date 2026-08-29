use atomic_write_file::AtomicWriteFile;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub const MAX_RECENT_FILES: usize = 10;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentFile {
    pub id: String,
    pub path: String,
    pub opened_at: u128,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentFilesEnvelope {
    pub schema_version: u32,
    pub entries: Vec<RecentFile>,
}

impl Default for RecentFilesEnvelope {
    fn default() -> Self {
        Self {
            schema_version: 1,
            entries: Vec::new(),
        }
    }
}

pub struct RecentFilesManager {
    path: PathBuf,
    state: Mutex<RecentFilesEnvelope>,
}

impl RecentFilesManager {
    pub fn load(path: PathBuf) -> io::Result<Self> {
        let state = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(io::Error::other)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => RecentFilesEnvelope::default(),
            Err(error) => return Err(error),
        };
        Ok(Self {
            path,
            state: Mutex::new(state),
        })
    }

    pub fn path_for_id(&self, id: &str) -> Option<String> {
        self.state
            .lock()
            .ok()?
            .entries
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.path.clone())
    }

    pub fn id_for_path(&self, path: &str) -> Option<String> {
        self.state
            .lock()
            .ok()?
            .entries
            .iter()
            .find(|entry| entry.path == path)
            .map(|entry| entry.id.clone())
    }

    pub fn record(&self, id: String, path: String) -> io::Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| io::Error::other("recent file state poisoned"))?;
        state.entries.retain(|entry| entry.path != path);
        state.entries.insert(
            0,
            RecentFile {
                id,
                path,
                opened_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(io::Error::other)?
                    .as_millis(),
            },
        );
        state.entries.truncate(MAX_RECENT_FILES);
        let bytes = serde_json::to_vec_pretty(&*state).map_err(io::Error::other)?;
        let mut file = AtomicWriteFile::options().open(&self.path)?;
        file.write_all(&bytes)?;
        file.commit()
    }
}
