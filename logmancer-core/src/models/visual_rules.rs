use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct VisualRule {
    pub matcher: VisualMatcher,
    pub case_sensitive: bool,
    pub style: LineStyleIntent,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum VisualMatcher {
    Text(String),
    Regex(String),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(transparent)]
/// UI-neutral color token carried by core.
///
/// Consumers must validate and map this value before rendering it as CSS,
/// terminal styles, or any other UI-specific color representation.
pub struct VisualColor(pub String);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct LineStyleIntent {
    pub foreground: Option<VisualColor>,
    pub background: Option<VisualColor>,
}
