use logmancer_core::{ManagedVisualRule, VisualRulesEnvelope};

#[derive(Clone, Debug)]
pub struct VisualRulesEditorState {
    baseline_revision: u64,
    baseline: VisualRulesEnvelope,
    envelope: VisualRulesEnvelope,
    status: String,
    drawer_is_open: bool,
    editor_index: Option<usize>,
    focus_target: VisualRulesFocusTarget,
    loaded_from_server: bool,
    operation_generation: u64,
    requires_replace: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisualRulesFocusTarget {
    Invoker,
    Drawer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisualRulesFocusRequest {
    Invoker,
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
impl VisualRulesEditorState {
    pub fn new(revision: u64, envelope: VisualRulesEnvelope) -> Self {
        Self {
            baseline_revision: revision,
            baseline: envelope.clone(),
            envelope,
            status: String::new(),
            drawer_is_open: false,
            editor_index: None,
            focus_target: VisualRulesFocusTarget::Invoker,
            loaded_from_server: false,
            operation_generation: 0,
            requires_replace: false,
        }
    }

    pub fn revision(&self) -> u64 {
        self.baseline_revision
    }

    pub fn envelope(&self) -> &VisualRulesEnvelope {
        &self.envelope
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn begin_operation(&mut self) -> u64 {
        self.operation_generation = self.operation_generation.wrapping_add(1);
        self.operation_generation
    }

    pub fn ordinary_save_allowed(&self) -> bool {
        !self.requires_replace
    }

    pub fn open_drawer(&mut self) {
        self.drawer_is_open = true;
        self.focus_target = VisualRulesFocusTarget::Drawer;
    }

    #[allow(dead_code)]
    pub fn drawer_is_open(&self) -> bool {
        self.drawer_is_open
    }

    pub fn open_editor(&mut self, index: usize) {
        self.editor_index = Some(index);
    }

    #[allow(dead_code)]
    pub fn editor_index(&self) -> Option<usize> {
        self.editor_index
    }

    pub fn close_editor_with_escape(&mut self) {
        self.editor_index = None;
        self.focus_target = VisualRulesFocusTarget::Drawer;
    }

    pub fn close_drawer_with_escape(&mut self) -> VisualRulesFocusRequest {
        self.drawer_is_open = false;
        self.editor_index = None;
        self.focus_target = VisualRulesFocusTarget::Invoker;
        VisualRulesFocusRequest::Invoker
    }

    #[allow(dead_code)]
    pub fn focus_target(&self) -> VisualRulesFocusTarget {
        self.focus_target
    }

    pub fn load_saved_once(&mut self, revision: u64, envelope: VisualRulesEnvelope) -> bool {
        if self.loaded_from_server {
            return false;
        }

        let has_local_edits = self.envelope != self.baseline;
        self.baseline_revision = revision;
        self.baseline = envelope.clone();
        if !has_local_edits {
            self.envelope = envelope;
        } else {
            self.requires_replace = true;
            self.status = "Draft preserved; use Replace or Discard before saving.".to_string();
        }
        self.loaded_from_server = true;
        true
    }

    pub fn reload_saved_for(
        &mut self,
        operation: u64,
        revision: u64,
        envelope: VisualRulesEnvelope,
        message: impl Into<String>,
    ) -> bool {
        if operation != self.operation_generation {
            return false;
        }
        let has_local_edits = self.envelope != self.baseline;
        self.baseline_revision = revision;
        self.baseline = envelope.clone();
        if !has_local_edits {
            self.envelope = envelope;
        }
        self.requires_replace = has_local_edits;
        self.status = message.into();
        if has_local_edits {
            self.status
                .push_str(" Draft preserved; use Replace or Discard before saving.");
        }
        self.loaded_from_server = true;
        true
    }

    pub fn add(&mut self, rule: ManagedVisualRule) {
        self.envelope.rules.push(rule);
    }

    pub fn replace_rule(&mut self, index: usize, rule: ManagedVisualRule) {
        if let Some(existing) = self.envelope.rules.get_mut(index) {
            *existing = rule;
        }
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.envelope.rules.len() {
            self.envelope.rules.remove(index);
        }
    }

    pub fn move_rule(&mut self, index: usize, direction: isize) {
        let target = index.checked_add_signed(direction);
        if let Some(target) = target.filter(|target| *target < self.envelope.rules.len()) {
            self.envelope.rules.swap(index, target);
        }
    }

    pub fn collapse(&mut self) {}

    pub fn discard(&mut self) {
        self.begin_operation();
        self.envelope = self.baseline.clone();
        self.requires_replace = false;
        self.status = "Discarded unsaved visual rule changes.".to_string();
    }

    pub fn save_failed(&mut self, message: impl Into<String>) {
        self.status = message.into();
    }

    pub fn save_succeeded(
        &mut self,
        revision: u64,
        envelope: VisualRulesEnvelope,
        message: impl Into<String>,
    ) {
        let operation = self.operation_generation;
        self.save_succeeded_for(operation, revision, envelope, message);
    }

    pub fn save_succeeded_for(
        &mut self,
        operation: u64,
        revision: u64,
        envelope: VisualRulesEnvelope,
        message: impl Into<String>,
    ) -> bool {
        if operation != self.operation_generation {
            return false;
        }
        let has_later_edits = self.envelope != envelope;
        self.baseline_revision = revision;
        self.baseline = envelope.clone();
        if !has_later_edits {
            self.envelope = envelope;
        }
        self.status = message.into();
        self.loaded_from_server = true;
        self.requires_replace = false;
        true
    }
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn operation_status(success: &str, diagnostics: &[String]) -> String {
    if diagnostics.is_empty() {
        success.to_string()
    } else {
        format!("Persistence warning: {}", diagnostics.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use logmancer_core::{
        LineStyleIntent, ManagedVisualRule, VisualColor, VisualMatcher, VisualRulesEnvelope,
    };

    fn rule(name: &str, pattern: &str) -> ManagedVisualRule {
        ManagedVisualRule {
            name: Some(name.to_string()),
            enabled: true,
            matcher: VisualMatcher::Text(pattern.to_string()),
            case_sensitive: false,
            style: LineStyleIntent {
                foreground: Some(VisualColor("red".to_string())),
                background: None,
            },
        }
    }

    #[test]
    fn local_edits_survive_collapse_and_discard_restores_baseline() {
        let baseline = VisualRulesEnvelope::new(vec![rule("Errors", "ERROR")]);
        let mut state = VisualRulesEditorState::new(7, baseline.clone());

        state.add(rule("Warnings", "WARN"));
        state.collapse();

        assert_eq!(state.envelope().rules.len(), 2);
        assert_eq!(state.envelope().rules[1].name.as_deref(), Some("Warnings"));

        state.discard();

        assert_eq!(state.revision(), 7);
        assert_eq!(state.envelope().rules, baseline.rules);
    }

    #[test]
    fn failed_save_retains_ordered_local_edits_and_success_updates_baseline() {
        let baseline = VisualRulesEnvelope::new(vec![rule("Errors", "ERROR")]);
        let mut state = VisualRulesEditorState::new(2, baseline);
        state.add(rule("Warnings", "WARN"));

        state.save_failed("Configuration changed elsewhere.");
        assert_eq!(state.envelope().rules.len(), 2);
        assert_eq!(state.status(), "Configuration changed elsewhere.");

        let saved = state.envelope().clone();
        state.save_succeeded(3, saved.clone(), "Saved visual rules.");
        state.add(rule("Debug", "DEBUG"));
        state.discard();

        assert_eq!(state.revision(), 3);
        assert_eq!(state.envelope().rules, saved.rules);
    }

    #[test]
    fn drawer_and_editor_escape_preserve_local_edits_and_restore_invoker() {
        let baseline = VisualRulesEnvelope::new(vec![rule("Errors", "ERROR")]);
        let mut state = VisualRulesEditorState::new(4, baseline);

        state.open_drawer();
        state.add(rule("Warnings", "WARN"));
        state.open_editor(1);
        state.close_editor_with_escape();

        assert_eq!(state.editor_index(), None);
        assert_eq!(state.focus_target(), VisualRulesFocusTarget::Drawer);
        assert_eq!(state.envelope().rules.len(), 2);

        state.close_drawer_with_escape();

        assert!(!state.drawer_is_open());
        assert_eq!(state.focus_target(), VisualRulesFocusTarget::Invoker);
        assert_eq!(state.envelope().rules[1].name.as_deref(), Some("Warnings"));
    }

    #[test]
    fn invalid_recovery_status_does_not_discard_the_editable_copy() {
        let baseline = VisualRulesEnvelope::new(vec![rule("Errors", "ERROR")]);
        let mut state = VisualRulesEditorState::new(1, baseline);
        state.add(rule("Broken draft", "["));

        state.save_failed("rule 2: invalid regex");

        assert_eq!(state.status(), "rule 2: invalid regex");
        assert_eq!(state.envelope().rules.len(), 2);
        assert_eq!(
            state.envelope().rules[1].name.as_deref(),
            Some("Broken draft")
        );
    }

    #[test]
    fn reopening_drawer_does_not_replace_an_unsaved_local_copy() {
        let mut state = VisualRulesEditorState::new(0, VisualRulesEnvelope::new(Vec::new()));
        let saved = VisualRulesEnvelope::new(vec![rule("Errors", "ERROR")]);

        assert!(state.load_saved_once(1, saved));
        state.add(rule("Warnings", "WARN"));
        state.close_drawer_with_escape();
        state.open_drawer();

        assert!(!state.load_saved_once(2, VisualRulesEnvelope::new(vec![rule("Other", "OTHER")])));
        assert_eq!(state.envelope().rules.len(), 2);
        assert_eq!(state.envelope().rules[1].name.as_deref(), Some("Warnings"));
    }

    #[test]
    fn delayed_initial_load_updates_baseline_without_replacing_local_edits() {
        let mut state = VisualRulesEditorState::new(0, VisualRulesEnvelope::new(Vec::new()));
        state.add(rule("Local", "LOCAL"));
        let saved = VisualRulesEnvelope::new(vec![rule("Saved", "SAVED")]);
        assert!(state.load_saved_once(5, saved.clone()));
        assert_eq!(state.revision(), 5);
        assert_eq!(state.envelope().rules[0].name.as_deref(), Some("Local"));
        assert!(!state.ordinary_save_allowed());
        assert!(state.status().contains("Replace or Discard"));
        state.discard();
        assert_eq!(state.envelope().rules, saved.rules);
        assert!(state.ordinary_save_allowed());
    }

    #[test]
    fn save_success_preserves_edits_made_after_submission() {
        let mut state = VisualRulesEditorState::new(1, VisualRulesEnvelope::new(Vec::new()));
        state.add(rule("Submitted", "ONE"));
        let submitted = state.envelope().clone();
        state.add(rule("Later", "TWO"));
        state.save_succeeded(2, submitted.clone(), "Saved visual rules.");
        assert_eq!(state.revision(), 2);
        assert_eq!(state.envelope().rules.len(), 2);
        state.discard();
        assert_eq!(state.envelope().rules, submitted.rules);
    }

    #[test]
    fn reload_rebases_revision_without_discarding_conflicting_draft() {
        let mut state = VisualRulesEditorState::new(1, VisualRulesEnvelope::new(Vec::new()));
        state.add(rule("Draft", "DRAFT"));
        let latest = VisualRulesEnvelope::new(vec![rule("Latest", "LATEST")]);
        let operation = state.begin_operation();
        state.reload_saved_for(operation, 4, latest.clone(), "Loaded visual rules.");
        assert_eq!(state.revision(), 4);
        assert_eq!(state.envelope().rules[0].name.as_deref(), Some("Draft"));
        assert!(!state.ordinary_save_allowed());
        assert!(state.status().contains("Replace or Discard"));
        state.discard();
        assert_eq!(state.envelope().rules, latest.rules);
        assert!(state.ordinary_save_allowed());
    }

    #[test]
    fn older_save_response_cannot_rollback_a_newer_reload() {
        let mut state = VisualRulesEditorState::new(1, VisualRulesEnvelope::new(Vec::new()));
        state.add(rule("Draft", "DRAFT"));
        let save_operation = state.begin_operation();
        let submitted = state.envelope().clone();
        let reload_operation = state.begin_operation();
        let latest = VisualRulesEnvelope::new(vec![rule("Latest", "LATEST")]);
        assert!(state.reload_saved_for(reload_operation, 4, latest, "Loaded visual rules.",));
        assert!(!state.save_succeeded_for(save_operation, 2, submitted, "Saved visual rules.",));
        assert_eq!(state.revision(), 4);
        assert!(!state.ordinary_save_allowed());
    }

    #[test]
    fn operation_status_surfaces_persistence_diagnostics() {
        assert_eq!(operation_status("", &[]), "");
        assert_eq!(
            operation_status("", &["directory sync failed".to_string()]),
            "Persistence warning: directory sync failed"
        );

        let mut state = VisualRulesEditorState::new(0, VisualRulesEnvelope::new(Vec::new()));
        state.save_failed("Could not load visual rules.");
        assert_eq!(state.status(), "Could not load visual rules.");
    }

    #[test]
    fn drawer_close_requests_focus_for_the_visual_rules_invoker() {
        let mut state = VisualRulesEditorState::new(0, VisualRulesEnvelope::new(Vec::new()));
        state.open_drawer();

        let focus_request = state.close_drawer_with_escape();

        assert_eq!(focus_request, VisualRulesFocusRequest::Invoker);
    }
}
