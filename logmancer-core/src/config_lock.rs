use crate::visual_rules_io::{
    VisualRulesIoError, VisualRulesPersistenceStage, committed_io_warning, contextual_io_error,
};
use crate::visual_rules_store::StoreCommit;
use std::fs::OpenOptions;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

static COMMIT_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub fn with_config_file_lock<T>(
    path: &Path,
    operation: impl FnOnce() -> io::Result<T>,
) -> io::Result<T> {
    let (value, unlock_warning) = with_config_file_lock_inner(path, operation)?;
    match unlock_warning {
        Some(error) => Err(error),
        None => Ok(value),
    }
}

pub(crate) fn with_config_file_lock_reconciled(
    path: &Path,
    operation: impl FnOnce() -> io::Result<StoreCommit>,
) -> io::Result<StoreCommit> {
    let (commit, unlock_warning) = with_config_file_lock_inner(path, operation)?;
    match unlock_warning {
        Some(error) => Err(committed_io_warning(
            commit,
            VisualRulesIoError::from_context(VisualRulesPersistenceStage::Unlock, path, error),
        )),
        None => Ok(commit),
    }
}

fn with_config_file_lock_inner<T>(
    path: &Path,
    operation: impl FnOnce() -> io::Result<T>,
) -> io::Result<(T, Option<io::Error>)> {
    let _commit = COMMIT_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| io::Error::other("configuration commit lock poisoned"))?;
    let mut lock_path = path.as_os_str().to_os_string();
    lock_path.push(".lock");
    let lock_path = PathBuf::from(lock_path);
    let lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|error| {
            contextual_io_error(VisualRulesPersistenceStage::LockOpen, &lock_path, error)
        })?;
    lock_file.lock().map_err(|error| {
        contextual_io_error(VisualRulesPersistenceStage::Lock, &lock_path, error)
    })?;
    let operation_result = operation();
    let unlock_result = lock_file.unlock().map_err(|error| {
        contextual_io_error(VisualRulesPersistenceStage::Unlock, &lock_path, error)
    });
    match operation_result {
        Ok(value) => Ok((value, unlock_result.err())),
        Err(error) => Err(error),
    }
}
