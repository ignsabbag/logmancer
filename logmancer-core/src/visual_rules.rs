use regex::{Regex, RegexBuilder};

use crate::models::visual_rules::{LineStyleIntent, VisualMatcher, VisualRule};

#[derive(Clone, Debug, Default)]
pub struct VisualRuleEvaluator {
    rules: Vec<CompiledVisualRule>,
}

#[derive(Clone, Debug)]
struct CompiledVisualRule {
    matcher: CompiledVisualMatcher,
    style: LineStyleIntent,
}

#[derive(Clone, Debug)]
enum CompiledVisualMatcher {
    Text {
        pattern: String,
        case_sensitive: bool,
    },
    Regex(Regex),
}

impl VisualRuleEvaluator {
    pub fn compile(rules: &[VisualRule]) -> Self {
        let rules = rules
            .iter()
            .filter_map(CompiledVisualRule::compile)
            .collect();

        Self { rules }
    }

    pub fn evaluate(&self, line: &str) -> Option<LineStyleIntent> {
        self.rules
            .iter()
            .find(|rule| rule.matches(line))
            .map(|rule| rule.style.clone())
    }
}

impl CompiledVisualRule {
    fn compile(rule: &VisualRule) -> Option<Self> {
        let matcher = match &rule.matcher {
            VisualMatcher::Text(pattern) => CompiledVisualMatcher::Text {
                pattern: normalize_text(pattern, rule.case_sensitive),
                case_sensitive: rule.case_sensitive,
            },
            VisualMatcher::Regex(pattern) => RegexBuilder::new(pattern)
                .case_insensitive(!rule.case_sensitive)
                .build()
                .ok()
                .map(CompiledVisualMatcher::Regex)?,
        };

        Some(Self {
            matcher,
            style: rule.style.clone(),
        })
    }

    fn matches(&self, line: &str) -> bool {
        match &self.matcher {
            CompiledVisualMatcher::Text {
                pattern,
                case_sensitive,
            } => normalize_text(line, *case_sensitive).contains(pattern),
            CompiledVisualMatcher::Regex(regex) => regex.is_match(line),
        }
    }
}

fn normalize_text(value: &str, case_sensitive: bool) -> String {
    if case_sensitive {
        value.to_string()
    } else {
        value.to_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LineStyleIntent, VisualColor, VisualMatcher, VisualRule};

    fn style(foreground: &str, background: &str) -> LineStyleIntent {
        LineStyleIntent {
            foreground: Some(VisualColor(foreground.to_string())),
            background: Some(VisualColor(background.to_string())),
        }
    }

    fn text_rule(pattern: &str, case_sensitive: bool, foreground: &str) -> VisualRule {
        VisualRule {
            matcher: VisualMatcher::Text(pattern.to_string()),
            case_sensitive,
            style: style(foreground, "default"),
        }
    }

    fn regex_rule(pattern: &str, case_sensitive: bool, foreground: &str) -> VisualRule {
        VisualRule {
            matcher: VisualMatcher::Regex(pattern.to_string()),
            case_sensitive,
            style: style(foreground, "default"),
        }
    }

    #[test]
    fn visual_rules_text_match_respects_case_control() {
        let insensitive = VisualRuleEvaluator::compile(&[text_rule("error", false, "red")]);
        assert_eq!(
            insensitive.evaluate("Error: disk full"),
            Some(style("red", "default"))
        );

        let sensitive = VisualRuleEvaluator::compile(&[text_rule("error", true, "red")]);
        assert_eq!(sensitive.evaluate("Error: disk full"), None);
    }

    #[test]
    fn visual_rules_regex_match_uses_valid_patterns() {
        let evaluator = VisualRuleEvaluator::compile(&[regex_rule("^WARN:", true, "yellow")]);

        assert_eq!(
            evaluator.evaluate("WARN: cache unavailable"),
            Some(style("yellow", "default"))
        );
        assert_eq!(evaluator.evaluate("INFO: cache unavailable"), None);
    }

    #[test]
    fn visual_rules_first_matching_rule_wins() {
        let evaluator = VisualRuleEvaluator::compile(&[
            text_rule("timeout", false, "orange"),
            text_rule("timeout", false, "blue"),
        ]);

        assert_eq!(
            evaluator.evaluate("request timeout"),
            Some(style("orange", "default"))
        );
        assert_eq!(
            evaluator.evaluate("request timeout"),
            Some(style("orange", "default"))
        );
    }

    #[test]
    fn visual_rules_invalid_regex_is_skipped_without_blocking_valid_rules() {
        let evaluator = VisualRuleEvaluator::compile(&[
            regex_rule("[", true, "broken"),
            text_rule("panic", false, "red"),
        ]);

        assert_eq!(
            evaluator.evaluate("thread panic observed"),
            Some(style("red", "default"))
        );
        assert_eq!(evaluator.evaluate("thread healthy"), None);
    }
}
