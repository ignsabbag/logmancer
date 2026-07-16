mod file_ops;
mod handler;
mod models;
mod reader;
mod registry;
mod timing;
mod visual_rules;
mod workers;

pub use models::file_info::FileInfo;
pub use models::page_result::{PageLine, PageResult};
pub use models::search::{PageSearchResult, SearchDisplayStatus, SearchMatch, SearchStatus};
pub use models::visual_rules::{LineStyleIntent, VisualColor, VisualMatcher, VisualRule};
pub use reader::LogReader;
pub use registry::LogRegistry;
pub use visual_rules::VisualRuleEvaluator;
