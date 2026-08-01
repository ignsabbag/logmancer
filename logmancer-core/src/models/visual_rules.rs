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

pub const MAX_STORED_VISUAL_RULES: usize = 100;
pub const MAX_ENABLED_VISUAL_RULES: usize = 50;
pub const MAX_VISUAL_RULE_PATTERN_LENGTH: usize = 512;
pub const MAX_VISUAL_RULE_NAME_LENGTH: usize = 80;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VisualRulesEnvelope {
    pub schema_version: u32,
    pub rules: Vec<ManagedVisualRule>,
}

impl VisualRulesEnvelope {
    pub const MAX_PERSISTED_SIZE: usize = 256 * 1024;

    pub fn new(rules: Vec<ManagedVisualRule>) -> Self {
        Self {
            schema_version: 1,
            rules,
        }
    }

    pub fn validate_for_save(&self) -> Result<ValidationReport, ValidationError> {
        validate_envelope(self, false)
    }

    pub fn validate_for_load(&self) -> Result<ValidationReport, ValidationError> {
        validate_envelope(self, true)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ManagedVisualRule {
    pub name: Option<String>,
    pub enabled: bool,
    pub matcher: VisualMatcher,
    pub case_sensitive: bool,
    pub style: LineStyleIntent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationReport {
    pub evaluator_rules: Vec<VisualRule>,
    pub diagnostics: Vec<ValidationDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationDiagnostic {
    pub severity: ValidationSeverity,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidationSeverity {
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationError {
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ValidationError {}

fn validate_envelope(
    envelope: &VisualRulesEnvelope,
    recover_entries: bool,
) -> Result<ValidationReport, ValidationError> {
    if envelope.schema_version != 1 {
        return Err(ValidationError {
            message: "unsupported schemaVersion".to_string(),
        });
    }
    if envelope.rules.len() > MAX_STORED_VISUAL_RULES {
        return Err(ValidationError {
            message: "at most 100 rules may be stored".to_string(),
        });
    }
    if envelope.rules.iter().filter(|rule| rule.enabled).count() > MAX_ENABLED_VISUAL_RULES {
        return Err(ValidationError {
            message: "at most 50 rules may be enabled".to_string(),
        });
    }

    let mut evaluator_rules = Vec::new();
    let mut diagnostics = Vec::new();
    for (index, rule) in envelope.rules.iter().enumerate() {
        match validate_rule(rule) {
            Ok(()) if rule.enabled => evaluator_rules.push(VisualRule {
                matcher: rule.matcher.clone(),
                case_sensitive: rule.case_sensitive,
                style: rule.style.clone(),
            }),
            Ok(()) => {}
            Err(message) if recover_entries => diagnostics.push(ValidationDiagnostic {
                severity: ValidationSeverity::Warning,
                message: format!("rule {}: {}", index + 1, message),
            }),
            Err(message) => {
                return Err(ValidationError {
                    message: format!("rule {}: {}", index + 1, message),
                });
            }
        }
    }
    Ok(ValidationReport {
        evaluator_rules,
        diagnostics,
    })
}

fn validate_rule(rule: &ManagedVisualRule) -> Result<(), String> {
    if rule
        .name
        .as_ref()
        .is_some_and(|name| name.chars().count() > MAX_VISUAL_RULE_NAME_LENGTH)
    {
        return Err("name exceeds 80 characters".to_string());
    }
    let pattern = match &rule.matcher {
        VisualMatcher::Text(pattern) | VisualMatcher::Regex(pattern) => pattern,
    };
    if pattern.chars().count() > MAX_VISUAL_RULE_PATTERN_LENGTH {
        return Err("pattern exceeds 512 characters".to_string());
    }
    for color in [&rule.style.foreground, &rule.style.background]
        .into_iter()
        .flatten()
    {
        if !matches!(
            color.0.as_str(),
            "default" | "red" | "orange" | "yellow" | "green" | "cyan" | "blue" | "purple" | "gray"
        ) {
            return Err(format!("unsupported palette token '{}'", color.0));
        }
    }
    if let VisualMatcher::Regex(pattern) = &rule.matcher
        && regex::RegexBuilder::new(pattern)
            .case_insensitive(!rule.case_sensitive)
            .build()
            .is_err()
    {
        return Err("invalid regex".to_string());
    }
    Ok(())
}
