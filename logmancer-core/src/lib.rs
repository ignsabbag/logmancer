#[cfg(feature = "native-persistence")]
mod config_store;
mod file_ops;
mod handler;
mod models;
mod reader;
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
