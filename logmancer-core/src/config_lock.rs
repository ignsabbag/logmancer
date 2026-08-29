use std::fs::OpenOptions;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

static COMMIT_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub fn with_config_file_lock<T>(
    path: &Path,
    operation: impl FnOnce() -> io::Result<T>,
) -> io::Result<T> {
    let _commit = COMMIT_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| io::Error::other("configuration commit lock poisoned"))?;
    let mut lock_path = path.as_os_str().to_os_string();
    lock_path.push(".lock");
    let lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(PathBuf::from(lock_path))?;
    lock_file.lock()?;
    match (operation(), lock_file.unlock()) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), _) | (_, Err(error)) => Err(error),
    }
}
