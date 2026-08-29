use crate::config_lock::with_config_file_lock;
use atomic_write_file::AtomicWriteFile;
use log::warn;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
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
        let state = read_envelope(&path)?;
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
        with_config_file_lock(&self.path, || {
            let mut merged = read_envelope(&self.path)?;
            apply_record(&mut merged, id, path)?;
            write_envelope(&self.path, &merged)?;
            *state = merged;
            Ok(())
        })
    }
}

fn read_envelope(path: &Path) -> io::Result<RecentFilesEnvelope> {
    match fs::read(path) {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(envelope) => Ok(envelope),
            Err(error) => recover_corrupt_file(path, error),
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(RecentFilesEnvelope::default()),
        Err(error) => Err(error),
    }
}

fn recover_corrupt_file(path: &Path, error: serde_json::Error) -> io::Result<RecentFilesEnvelope> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_millis();
    let backup = path.with_file_name(format!("recent-files.{timestamp}.corrupt.json"));
    fs::copy(path, &backup)?;
    let envelope = RecentFilesEnvelope::default();
    write_envelope(path, &envelope)?;
    warn!(
        "Recovered corrupt recent-files history at {} into {}: {}",
        path.display(),
        backup.display(),
        error
    );
    Ok(envelope)
}

fn apply_record(envelope: &mut RecentFilesEnvelope, id: String, path: String) -> io::Result<()> {
    envelope.entries.retain(|entry| entry.path != path);
    envelope.entries.insert(
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
    envelope.entries.truncate(MAX_RECENT_FILES);
    Ok(())
}

fn write_envelope(path: &Path, envelope: &RecentFilesEnvelope) -> io::Result<()> {
    let bytes = serde_json::to_vec_pretty(envelope).map_err(io::Error::other)?;
    let mut file = AtomicWriteFile::options().open(path)?;
    file.write_all(&bytes)?;
    file.commit()
}
