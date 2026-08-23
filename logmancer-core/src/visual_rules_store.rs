#![cfg(feature = "native-persistence")]

use crate::models::visual_rules::VisualRulesEnvelope;
use atomic_write_file::AtomicWriteFile;
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMPORARY_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static COMMIT_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
const OVERSIZED_TOKEN_DOMAIN: &[u8] = b"logmancer:visual-rules:oversized:sha256:v1\0";
const MAX_RETAINED_BACKUPS: usize = 10;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoreCommit {
    Committed,
    CommittedWithWarning(String),
}

impl StoreCommit {
    pub fn with_warning(message: impl Into<String>) -> Self {
        Self::CommittedWithWarning(message.into())
    }
}

pub trait VisualRulesStore: Send + Sync {
    fn read(&self) -> io::Result<Option<Vec<u8>>>;
    fn save_new(&self, bytes: &[u8]) -> io::Result<StoreCommit>;
    fn replace(&self, bytes: &[u8]) -> io::Result<StoreCommit>;
    fn compare_and_commit(
        &self,
        expected: Option<&[u8]>,
        bytes: &[u8],
        replace: bool,
    ) -> io::Result<StoreCommit>;
}

pub trait AtomicFileReplacer: Send + Sync {
    fn save_new(&self, path: &Path, bytes: &[u8]) -> io::Result<StoreCommit>;
    fn replace(&self, path: &Path, bytes: &[u8]) -> io::Result<StoreCommit>;
}

#[derive(Clone)]
pub struct NativeVisualRulesStore {
    path: PathBuf,
    replacer: Arc<dyn AtomicFileReplacer>,
}

impl NativeVisualRulesStore {
    pub fn new(path: PathBuf) -> Self {
        Self::with_replacer(path, Arc::new(NativeAtomicFileReplacer))
    }

    pub fn with_replacer(path: PathBuf, replacer: Arc<dyn AtomicFileReplacer>) -> Self {
        Self { path, replacer }
    }
}

impl VisualRulesStore for NativeVisualRulesStore {
    fn read(&self) -> io::Result<Option<Vec<u8>>> {
        let mut file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let read_limit = VisualRulesEnvelope::MAX_PERSISTED_SIZE + 1;
        let mut bytes = Vec::with_capacity(read_limit);
        (&mut file)
            .take(read_limit as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() <= VisualRulesEnvelope::MAX_PERSISTED_SIZE {
            return Ok(Some(bytes));
        }

        let mut digest = Sha256::new();
        digest.update(&bytes);
        let mut total_len = bytes.len() as u64;
        let mut buffer = [0_u8; 8192];
        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            digest.update(&buffer[..read]);
            total_len = total_len
                .checked_add(read as u64)
                .ok_or_else(|| io::Error::other("visual rules source length overflow"))?;
        }

        bytes.clear();
        bytes.extend_from_slice(OVERSIZED_TOKEN_DOMAIN);
        bytes.extend_from_slice(&total_len.to_be_bytes());
        bytes.extend_from_slice(&digest.finalize());
        bytes.resize(read_limit, 0);
        Ok(Some(bytes))
    }

    fn save_new(&self, bytes: &[u8]) -> io::Result<StoreCommit> {
        self.replacer.save_new(&self.path, bytes)
    }

    fn replace(&self, bytes: &[u8]) -> io::Result<StoreCommit> {
        self.replacer.replace(&self.path, bytes)
    }

    fn compare_and_commit(
        &self,
        expected: Option<&[u8]>,
        bytes: &[u8],
        replace: bool,
    ) -> io::Result<StoreCommit> {
        let _commit = COMMIT_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .map_err(|_| io::Error::other("visual rules commit lock poisoned"))?;
        let mut lock_path = self.path.as_os_str().to_os_string();
        lock_path.push(".lock");
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(PathBuf::from(lock_path))?;
        lock_file.lock()?;
        let result = (|| {
            if self.read()?.as_deref() != expected {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "visual rules source changed before publication",
                ));
            }
            if replace {
                self.replace(bytes)
            } else {
                self.save_new(bytes)
            }
        })();
        match (result, lock_file.unlock()) {
            (Ok(commit), Ok(())) => Ok(commit),
            (Err(error), _) | (_, Err(error)) => Err(error),
        }
    }
}

#[derive(Clone, Debug)]
pub struct NativeAtomicFileReplacer;

impl AtomicFileReplacer for NativeAtomicFileReplacer {
    #[cfg(unix)]
    fn save_new(&self, path: &Path, bytes: &[u8]) -> io::Result<StoreCommit> {
        let temporary_path = temporary_path(path);
        let mut temporary = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)?;
        temporary.write_all(bytes)?;
        temporary.sync_all()?;
        drop(temporary);

        if let Err(error) = fs::hard_link(&temporary_path, path) {
            let _ = fs::remove_file(&temporary_path);
            return Err(error);
        }

        let cleanup = fs::remove_file(&temporary_path).and_then(|_| sync_parent(path));
        match cleanup {
            Ok(()) => Ok(StoreCommit::Committed),
            Err(error) => Ok(StoreCommit::with_warning(error.to_string())),
        }
    }

    #[cfg(windows)]
    fn save_new(&self, path: &Path, bytes: &[u8]) -> io::Result<StoreCommit> {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_WRITE_THROUGH, MoveFileExW};

        let temporary_path = temporary_path(path);
        let mut temporary = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)?;
        temporary.write_all(bytes)?;
        temporary.sync_all()?;
        drop(temporary);

        let source: Vec<u16> = temporary_path
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        let target: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe { MoveFileExW(source.as_ptr(), target.as_ptr(), MOVEFILE_WRITE_THROUGH) } == 0 {
            let error = io::Error::last_os_error();
            let _ = fs::remove_file(&temporary_path);
            return Err(error);
        }
        Ok(StoreCommit::Committed)
    }

    #[cfg(not(any(unix, windows)))]
    fn save_new(&self, _path: &Path, _bytes: &[u8]) -> io::Result<StoreCommit> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "native first-save unsupported",
        ))
    }

    fn replace(&self, path: &Path, bytes: &[u8]) -> io::Result<StoreCommit> {
        let backup = timestamped_backup_path(path)?;
        fs::copy(path, &backup)?;
        File::open(&backup)?.sync_all()?;

        let mut file = AtomicWriteFile::options().open(path)?;
        file.write_all(bytes)?;
        file.commit()?;
        let sync_result = sync_parent(path);
        // Retention is best-effort: a cleanup failure must not undo a committed update.
        let _ = prune_backups(path);
        match sync_result {
            Ok(()) => Ok(StoreCommit::Committed),
            Err(error) => Ok(StoreCommit::with_warning(error.to_string())),
        }
    }
}

fn temporary_path(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let sequence = TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    parent.join(format!(
        ".{}.{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("visual-rules"),
        std::process::id(),
        sequence
    ))
}

fn timestamped_backup_path(path: &Path) -> io::Result<PathBuf> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_millis();
    Ok(path.with_extension(format!("{timestamp}.bak")))
}

fn prune_backups(path: &Path) -> io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("visual-rules");
    let prefix = format!("{stem}.");
    let mut backups = Vec::new();

    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Some(timestamp) = name
            .strip_prefix(&prefix)
            .and_then(|name| name.strip_suffix(".bak"))
            .and_then(|timestamp| timestamp.parse::<u128>().ok())
        else {
            continue;
        };
        backups.push((timestamp, entry.path()));
    }

    backups.sort_unstable_by_key(|(timestamp, _)| *timestamp);
    let expired = backups.len().saturating_sub(MAX_RETAINED_BACKUPS);
    for (_, backup) in backups.into_iter().take(expired) {
        fs::remove_file(backup)?;
    }
    Ok(())
}

#[cfg(unix)]
fn sync_parent(path: &Path) -> io::Result<()> {
    File::open(path.parent().unwrap_or_else(|| Path::new(".")))?.sync_all()
}

#[cfg(not(unix))]
fn sync_parent(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pruning_retains_recent_backups_without_touching_unrelated_files() {
        let directory = std::env::temp_dir().join(format!(
            "logmancer-visual-rules-backups-{}-{}",
            std::process::id(),
            TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).expect("create temporary directory");
        let config = directory.join("visual-rules.json");
        let unrelated = directory.join("other.1.bak");
        fs::write(&unrelated, "unrelated backup").expect("write unrelated backup");

        for timestamp in 1..=12 {
            fs::write(config.with_extension(format!("{timestamp}.bak")), "backup")
                .expect("write visual rules backup");
        }

        prune_backups(&config).expect("prune backups");

        for timestamp in 1..=2 {
            assert!(!config.with_extension(format!("{timestamp}.bak")).exists());
        }
        for timestamp in 3..=12 {
            assert!(config.with_extension(format!("{timestamp}.bak")).exists());
        }
        assert!(unrelated.exists());

        fs::remove_dir_all(directory).expect("remove temporary directory");
    }
}
