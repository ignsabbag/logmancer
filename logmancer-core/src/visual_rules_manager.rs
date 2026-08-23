use crate::models::visual_rules::{ValidationDiagnostic, VisualRulesEnvelope};
use crate::visual_rules::VisualRuleEvaluator;
#[cfg(feature = "native-persistence")]
use crate::visual_rules_store::{StoreCommit, VisualRulesStore};
use std::sync::{Arc, Mutex, RwLock};

#[cfg(feature = "native-persistence")]
const PERSISTED_SIZE_ERROR: &str = "visual rules configuration exceeds 256 KiB";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveOutcome {
    Committed,
    CommittedWithWarning(String),
}

#[derive(Clone, Debug)]
pub struct VisualRulesState {
    pub revision: u64,
    pub envelope: VisualRulesEnvelope,
    pub diagnostics: Vec<ValidationDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveResult {
    pub revision: u64,
    pub outcome: SaveOutcome,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VisualRulesError {
    Validation(String),
    RevisionConflict,
    SourceConflict,
    Io(String),
    Decode(String),
}

impl VisualRulesError {
    pub fn is_source_conflict(&self) -> bool {
        matches!(self, Self::SourceConflict)
    }
    pub fn is_io(&self) -> bool {
        matches!(self, Self::Io(_))
    }
}

impl std::fmt::Display for VisualRulesError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for VisualRulesError {}

struct ManagerState {
    revision: u64,
    envelope: VisualRulesEnvelope,
    source: Option<Vec<u8>>,
    diagnostics: Vec<ValidationDiagnostic>,
}

pub struct VisualRulesManager {
    evaluator: RwLock<Arc<VisualRuleEvaluator>>,
    state: Mutex<ManagerState>,
    #[cfg(feature = "native-persistence")]
    store: Option<Arc<dyn VisualRulesStore>>,
}

impl VisualRulesManager {
    pub fn in_memory() -> Arc<Self> {
        Arc::new(Self {
            evaluator: RwLock::new(Arc::new(VisualRuleEvaluator::default())),
            state: Mutex::new(ManagerState {
                revision: 0,
                envelope: VisualRulesEnvelope::new(Vec::new()),
                source: None,
                diagnostics: Vec::new(),
            }),
            #[cfg(feature = "native-persistence")]
            store: None,
        })
    }

    #[cfg(feature = "native-persistence")]
    pub fn with_store(store: Arc<dyn VisualRulesStore>) -> Arc<Self> {
        let manager = match Arc::try_unwrap(Self::in_memory()) {
            Ok(manager) => manager,
            Err(_) => unreachable!("new manager has one owner"),
        };
        Arc::new(Self {
            store: Some(store),
            ..manager
        })
    }

    pub fn snapshot(&self) -> Arc<VisualRuleEvaluator> {
        self.evaluator
            .read()
            .expect("visual rules evaluator lock")
            .clone()
    }

    pub fn state(&self) -> VisualRulesState {
        let state = self.state.lock().expect("visual rules state lock");
        VisualRulesState {
            revision: state.revision,
            envelope: state.envelope.clone(),
            diagnostics: state.diagnostics.clone(),
        }
    }

    pub fn apply_memory(
        &self,
        envelope: VisualRulesEnvelope,
    ) -> Result<SaveResult, VisualRulesError> {
        let report = envelope
            .validate_for_save()
            .map_err(|error| VisualRulesError::Validation(error.message))?;
        let mut state = self.state.lock().expect("visual rules state lock");
        self.publish(
            &mut state,
            envelope,
            report.evaluator_rules,
            report.diagnostics,
            None,
        );
        Ok(SaveResult {
            revision: state.revision,
            outcome: SaveOutcome::Committed,
        })
    }

    #[cfg(feature = "native-persistence")]
    pub fn load(&self) -> Result<VisualRulesState, VisualRulesError> {
        let mut state = self.state.lock().expect("visual rules state lock");
        let source = self
            .store
            .as_ref()
            .expect("native store")
            .read()
            .map_err(io_error)?;
        if source
            .as_ref()
            .is_some_and(|bytes| bytes.len() > VisualRulesEnvelope::MAX_PERSISTED_SIZE)
        {
            state.revision += 1;
            state.source = source;
            state.diagnostics = vec![ValidationDiagnostic {
                severity: crate::models::ValidationSeverity::Warning,
                message: PERSISTED_SIZE_ERROR.to_string(),
            }];
            return Ok(VisualRulesState {
                revision: state.revision,
                envelope: state.envelope.clone(),
                diagnostics: state.diagnostics.clone(),
            });
        }
        let (envelope, report) = match source.as_deref() {
            None | Some([]) => (VisualRulesEnvelope::new(Vec::new()), None),
            Some(bytes) => match serde_json::from_slice::<VisualRulesEnvelope>(bytes)
                .map_err(|error| error.to_string())
                .and_then(|envelope| {
                    envelope
                        .validate_for_load()
                        .map_err(|error| error.message)
                        .map(|report| (envelope, report))
                }) {
                Ok((envelope, report)) => (envelope, Some(report)),
                Err(message) => {
                    state.revision += 1;
                    state.source = source;
                    state.diagnostics = vec![ValidationDiagnostic {
                        severity: crate::models::ValidationSeverity::Warning,
                        message,
                    }];
                    return Ok(VisualRulesState {
                        revision: state.revision,
                        envelope: state.envelope.clone(),
                        diagnostics: state.diagnostics.clone(),
                    });
                }
            },
        };
        let evaluator_rules = report
            .as_ref()
            .map(|report| report.evaluator_rules.clone())
            .unwrap_or_default();
        let diagnostics = report.map(|report| report.diagnostics).unwrap_or_default();
        self.publish(&mut state, envelope, evaluator_rules, diagnostics, source);
        Ok(VisualRulesState {
            revision: state.revision,
            envelope: state.envelope.clone(),
            diagnostics: state.diagnostics.clone(),
        })
    }

    #[cfg(feature = "native-persistence")]
    pub fn save(
        &self,
        base_revision: u64,
        envelope: VisualRulesEnvelope,
    ) -> Result<SaveResult, VisualRulesError> {
        self.persist(base_revision, envelope, Some(false))
    }

    #[cfg(feature = "native-persistence")]
    pub fn replace(
        &self,
        base_revision: u64,
        envelope: VisualRulesEnvelope,
    ) -> Result<SaveResult, VisualRulesError> {
        self.persist(base_revision, envelope, Some(true))
    }

    #[cfg(feature = "native-persistence")]
    pub fn upsert(
        &self,
        base_revision: u64,
        envelope: VisualRulesEnvelope,
    ) -> Result<SaveResult, VisualRulesError> {
        self.persist(base_revision, envelope, None)
    }

    #[cfg(feature = "native-persistence")]
    fn persist(
        &self,
        base_revision: u64,
        envelope: VisualRulesEnvelope,
        replace: Option<bool>,
    ) -> Result<SaveResult, VisualRulesError> {
        let report = envelope
            .validate_for_save()
            .map_err(|error| VisualRulesError::Validation(error.message))?;
        let bytes = serde_json::to_vec_pretty(&envelope)
            .map_err(|error| VisualRulesError::Decode(error.to_string()))?;
        if bytes.len() > VisualRulesEnvelope::MAX_PERSISTED_SIZE {
            return Err(VisualRulesError::Validation(
                PERSISTED_SIZE_ERROR.to_string(),
            ));
        }
        let mut state = self.state.lock().expect("visual rules state lock");
        if state.revision != base_revision {
            return Err(VisualRulesError::RevisionConflict);
        }
        let replace = replace.unwrap_or(state.source.is_some());
        let store = self.store.as_ref().expect("native store");
        let commit = store
            .compare_and_commit(state.source.as_deref(), &bytes, replace)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    VisualRulesError::SourceConflict
                } else {
                    io_error(error)
                }
            })?;
        self.publish(
            &mut state,
            envelope,
            report.evaluator_rules,
            report.diagnostics,
            Some(bytes),
        );
        Ok(SaveResult {
            revision: state.revision,
            outcome: match commit {
                StoreCommit::Committed => SaveOutcome::Committed,
                StoreCommit::CommittedWithWarning(message) => {
                    SaveOutcome::CommittedWithWarning(message)
                }
            },
        })
    }

    fn publish(
        &self,
        state: &mut ManagerState,
        envelope: VisualRulesEnvelope,
        rules: Vec<crate::models::VisualRule>,
        diagnostics: Vec<ValidationDiagnostic>,
        source: Option<Vec<u8>>,
    ) {
        *self.evaluator.write().expect("visual rules evaluator lock") =
            Arc::new(VisualRuleEvaluator::compile(&rules));
        state.envelope = envelope;
        state.diagnostics = diagnostics;
        state.source = source;
        state.revision += 1;
    }
}

#[cfg(feature = "native-persistence")]
fn io_error(error: std::io::Error) -> VisualRulesError {
    VisualRulesError::Io(error.to_string())
}

#[cfg(all(test, feature = "native-persistence"))]
mod tests {
    use super::*;
    use crate::{
        LineStyleIntent, ManagedVisualRule, VisualColor, VisualMatcher, VisualRulesEnvelope,
    };
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::{self, Receiver, Sender};
    use std::time::Duration;

    struct PausingReadStore {
        bytes: Mutex<Option<Vec<u8>>>,
        pause_next_read: AtomicBool,
        read_captured: Sender<()>,
        release_read: Mutex<Receiver<()>>,
        commit_finished: Sender<()>,
    }

    impl VisualRulesStore for PausingReadStore {
        fn read(&self) -> std::io::Result<Option<Vec<u8>>> {
            let bytes = self.bytes.lock().expect("store lock").clone();
            if self.pause_next_read.swap(false, Ordering::SeqCst) {
                self.read_captured.send(()).expect("report captured read");
                self.release_read
                    .lock()
                    .expect("release receiver lock")
                    .recv_timeout(Duration::from_secs(5))
                    .expect("captured read was not released before timeout");
            }
            Ok(bytes)
        }

        fn save_new(&self, _bytes: &[u8]) -> std::io::Result<StoreCommit> {
            unreachable!("the test replaces an existing source")
        }

        fn replace(&self, bytes: &[u8]) -> std::io::Result<StoreCommit> {
            *self.bytes.lock().expect("store lock") = Some(bytes.to_vec());
            Ok(StoreCommit::Committed)
        }

        fn compare_and_commit(
            &self,
            expected: Option<&[u8]>,
            bytes: &[u8],
            replace: bool,
        ) -> std::io::Result<StoreCommit> {
            let mut stored = self.bytes.lock().expect("store lock");
            if stored.as_deref() != expected {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "source changed before publication",
                ));
            }
            assert!(replace, "the test replaces an existing source");
            *stored = Some(bytes.to_vec());
            self.commit_finished.send(()).expect("report commit");
            Ok(StoreCommit::Committed)
        }
    }

    fn envelope(pattern: &str) -> VisualRulesEnvelope {
        VisualRulesEnvelope::new(vec![ManagedVisualRule {
            name: Some("test rule".to_string()),
            enabled: true,
            matcher: VisualMatcher::Text(pattern.to_string()),
            case_sensitive: false,
            style: LineStyleIntent {
                foreground: Some(VisualColor("red".to_string())),
                background: Some(VisualColor("default".to_string())),
            },
        }])
    }

    #[test]
    fn concurrent_load_cannot_overwrite_a_successful_save_in_memory() {
        let initial_envelope = envelope("ERROR");
        let saved_envelope = envelope("WARN");
        let (read_captured_tx, read_captured_rx) = mpsc::channel();
        let (release_read_tx, release_read_rx) = mpsc::channel();
        let (commit_finished_tx, commit_finished_rx) = mpsc::channel();
        let store = Arc::new(PausingReadStore {
            bytes: Mutex::new(Some(
                serde_json::to_vec(&initial_envelope).expect("serialize initial envelope"),
            )),
            pause_next_read: AtomicBool::new(false),
            read_captured: read_captured_tx,
            release_read: Mutex::new(release_read_rx),
            commit_finished: commit_finished_tx,
        });
        let manager = VisualRulesManager::with_store(store.clone());
        let initial = manager.load().expect("initial load");

        store.pause_next_read.store(true, Ordering::SeqCst);
        let loading_manager = manager.clone();
        let load_thread = std::thread::spawn(move || loading_manager.load());
        read_captured_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("concurrent load did not capture its source");

        let load_holds_state = manager.state.try_lock().is_err();
        let save_revision = initial.revision + u64::from(load_holds_state);
        let saving_manager = manager.clone();
        let save_thread =
            std::thread::spawn(move || saving_manager.replace(save_revision, saved_envelope));

        if load_holds_state {
            release_read_tx.send(()).expect("release captured read");
            commit_finished_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("save did not commit after load publication");
        } else {
            commit_finished_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("save did not commit while stale load was paused");
            release_read_tx.send(()).expect("release captured read");
        }

        let saved = save_thread
            .join()
            .expect("save thread")
            .expect("concurrent save");
        load_thread
            .join()
            .expect("load thread")
            .expect("concurrent load");

        let state = manager.state();
        assert_eq!(state.revision, saved.revision);
        assert_eq!(state.envelope, envelope("WARN"));
        assert!(manager.snapshot().evaluate("WARN").is_some());
        assert_eq!(manager.snapshot().evaluate("ERROR"), None);
        let persisted = store
            .bytes
            .lock()
            .expect("store lock")
            .clone()
            .expect("persisted rules");
        assert!(
            std::str::from_utf8(&persisted)
                .expect("persisted rules are UTF-8 JSON")
                .contains("\n  \"schemaVersion\"")
        );
    }
}
