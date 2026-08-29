#[cfg(feature = "native-persistence")]
mod config_lock;
#[cfg(feature = "native-persistence")]
mod config_store;
mod file_ops;
mod handler;
mod models;
mod reader;
#[cfg(feature = "native-persistence")]
mod recent_files;
mod registry;
mod timing;
mod visual_rules;
mod visual_rules_manager;
#[cfg(feature = "native-persistence")]
mod visual_rules_store;
mod workers;

#[cfg(feature = "native-persistence")]
pub use config_store::ConfigStore;
pub use models::file_info::FileInfo;
pub use models::page_result::{PageLine, PageResult};
pub use models::search::{PageSearchResult, SearchDisplayStatus, SearchMatch, SearchStatus};
pub use models::visual_rules::{
    LineStyleIntent, ManagedVisualRule, ValidationDiagnostic, ValidationError, ValidationReport,
    ValidationSeverity, VisualColor, VisualMatcher, VisualRule, VisualRulesEnvelope,
};
pub use reader::LogReader;
#[cfg(feature = "native-persistence")]
pub use recent_files::{MAX_RECENT_FILES, RecentFile, RecentFilesEnvelope, RecentFilesManager};
pub use registry::{FileOpenPolicy, LogRegistry, LogRegistryBuilder};
pub use visual_rules::VisualRuleEvaluator;
pub use visual_rules_manager::{
    SaveOutcome, SaveResult, VisualRulesError, VisualRulesManager, VisualRulesState,
};
#[cfg(feature = "native-persistence")]
pub use visual_rules_store::{
    AtomicFileReplacer, NativeAtomicFileReplacer, NativeVisualRulesStore, StoreCommit,
    VisualRulesStore,
};
