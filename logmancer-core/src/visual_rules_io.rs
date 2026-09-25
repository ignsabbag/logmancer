use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisualRulesPersistenceStage {
    LockOpen,
    Lock,
    Unlock,
    SourceOpen,
    SourceRead,
    BackupPath,
    BackupCopy,
    BackupOpen,
    BackupSync,
    TemporaryOpen,
    TemporaryWrite,
    TemporarySync,
    AtomicCommit,
    ParentSync,
    Cleanup,
}

impl VisualRulesPersistenceStage {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LockOpen => "lock_open",
            Self::Lock => "lock",
            Self::Unlock => "unlock",
            Self::SourceOpen => "source_open",
            Self::SourceRead => "source_read",
            Self::BackupPath => "backup_path",
            Self::BackupCopy => "backup_copy",
            Self::BackupOpen => "backup_open",
            Self::BackupSync => "backup_sync",
            Self::TemporaryOpen => "temporary_open",
            Self::TemporaryWrite => "temporary_write",
            Self::TemporarySync => "temporary_sync",
            Self::AtomicCommit => "atomic_commit",
            Self::ParentSync => "parent_sync",
            Self::Cleanup => "cleanup",
        }
    }
}

impl std::fmt::Display for VisualRulesPersistenceStage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[cfg(feature = "native-persistence")]
#[derive(Debug)]
pub(crate) struct ContextualIoError {
    stage: VisualRulesPersistenceStage,
    path: PathBuf,
    source: io::Error,
}

#[cfg(feature = "native-persistence")]
impl std::fmt::Display for ContextualIoError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} failed for {}: {}",
            self.stage,
            self.path.display(),
            self.source
        )
    }
}

#[cfg(feature = "native-persistence")]
impl Error for ContextualIoError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

#[cfg(feature = "native-persistence")]
pub(crate) fn contextual_io_error(
    stage: VisualRulesPersistenceStage,
    path: impl Into<PathBuf>,
    source: io::Error,
) -> io::Error {
    io::Error::new(
        source.kind(),
        ContextualIoError {
            stage,
            path: path.into(),
            source,
        },
    )
}

#[cfg(feature = "native-persistence")]
#[derive(Debug)]
struct SourceConflictError {
    path: PathBuf,
}

#[cfg(feature = "native-persistence")]
impl std::fmt::Display for SourceConflictError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "visual rules source changed before publication: {}",
            self.path.display()
        )
    }
}

#[cfg(feature = "native-persistence")]
impl Error for SourceConflictError {}

#[cfg(feature = "native-persistence")]
pub(crate) fn source_conflict_error(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::AlreadyExists,
        SourceConflictError {
            path: path.to_path_buf(),
        },
    )
}

#[cfg(feature = "native-persistence")]
pub(crate) fn is_source_conflict(error: &io::Error) -> bool {
    error
        .get_ref()
        .is_some_and(|source| source.downcast_ref::<SourceConflictError>().is_some())
}

#[cfg(feature = "native-persistence")]
#[derive(Debug)]
struct CommittedIoWarning {
    commit: crate::visual_rules_store::StoreCommit,
    warning: VisualRulesIoError,
}

#[cfg(feature = "native-persistence")]
impl std::fmt::Display for CommittedIoWarning {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "commit completed with warning: {}", self.warning)
    }
}

#[cfg(feature = "native-persistence")]
impl Error for CommittedIoWarning {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.warning)
    }
}

#[cfg(feature = "native-persistence")]
pub(crate) fn committed_io_warning(
    commit: crate::visual_rules_store::StoreCommit,
    warning: VisualRulesIoError,
) -> io::Error {
    io::Error::new(warning.kind(), CommittedIoWarning { commit, warning })
}

#[cfg(feature = "native-persistence")]
pub(crate) fn into_committed_io_warning(
    error: io::Error,
) -> Result<(crate::visual_rules_store::StoreCommit, VisualRulesIoError), io::Error> {
    if error
        .get_ref()
        .is_none_or(|source| source.downcast_ref::<CommittedIoWarning>().is_none())
    {
        return Err(error);
    }
    let warning = error
        .into_inner()
        .expect("committed warning has an inner error")
        .downcast::<CommittedIoWarning>()
        .expect("committed warning type was checked");
    Ok((warning.commit, warning.warning))
}

#[derive(Clone, Debug)]
pub struct VisualRulesIoError {
    stage: VisualRulesPersistenceStage,
    path: PathBuf,
    kind: io::ErrorKind,
    raw_os_error: Option<i32>,
    source: Arc<io::Error>,
}

impl VisualRulesIoError {
    pub fn new(
        stage: VisualRulesPersistenceStage,
        path: impl Into<PathBuf>,
        source: io::Error,
    ) -> Self {
        let kind = source.kind();
        let raw_os_error = find_raw_os_error(&source);
        Self {
            stage,
            path: path.into(),
            kind,
            raw_os_error,
            source: Arc::new(source),
        }
    }

    #[cfg(feature = "native-persistence")]
    pub(crate) fn from_context(
        default_stage: VisualRulesPersistenceStage,
        default_path: &Path,
        source: io::Error,
    ) -> Self {
        let (stage, path) = source
            .get_ref()
            .and_then(|error| error.downcast_ref::<ContextualIoError>())
            .map(|error| (error.stage, error.path.clone()))
            .unwrap_or((default_stage, default_path.to_path_buf()));
        Self::new(stage, path, source)
    }

    pub fn stage(&self) -> VisualRulesPersistenceStage {
        self.stage
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn kind(&self) -> io::ErrorKind {
        self.kind
    }

    pub fn raw_os_error(&self) -> Option<i32> {
        self.raw_os_error
    }

    pub fn causal_chain(&self) -> String {
        let mut messages = vec![self.to_string()];
        let mut source = Error::source(self);
        while let Some(error) = source {
            messages.push(error.to_string());
            source = error.source();
        }
        messages.join(": ")
    }
}

impl std::fmt::Display for VisualRulesIoError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} failed for {}: {}",
            self.stage,
            self.path.display(),
            self.source
        )
    }
}

impl Error for VisualRulesIoError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}

impl PartialEq for VisualRulesIoError {
    fn eq(&self, other: &Self) -> bool {
        self.stage == other.stage
            && self.path == other.path
            && self.kind == other.kind
            && self.raw_os_error == other.raw_os_error
            && self.causal_chain() == other.causal_chain()
    }
}

impl Eq for VisualRulesIoError {}

fn find_raw_os_error(error: &io::Error) -> Option<i32> {
    if let Some(code) = error.raw_os_error() {
        return Some(code);
    }
    let mut source = error.source();
    while let Some(current) = source {
        if let Some(io_error) = current.downcast_ref::<io::Error>()
            && let Some(code) = io_error.raw_os_error()
        {
            return Some(code);
        }
        source = current.source();
    }
    None
}
