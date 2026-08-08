use leptos::prelude::*;
use logmancer_core::{LineStyleIntent, ManagedVisualRule, VisualColor, VisualMatcher};

const VISUAL_COLOR_PALETTE: [(&str, &str); 9] = [
    ("default", "Default"),
    ("red", "Red"),
    ("orange", "Orange"),
    ("yellow", "Yellow"),
    ("green", "Green"),
    ("cyan", "Cyan"),
    ("blue", "Blue"),
    ("purple", "Purple"),
    ("gray", "Gray"),
];

#[derive(Clone, Copy)]
enum VisualColorRole {
    Foreground,
    Background,
}

pub fn new_rule() -> ManagedVisualRule {
    ManagedVisualRule {
        name: Some("New rule".to_string()),
        enabled: true,
        matcher: VisualMatcher::Text(String::new()),
        case_sensitive: false,
        style: LineStyleIntent {
            foreground: None,
            background: None,
        },
    }
}

fn updated_matcher_pattern(matcher: VisualMatcher, value: String) -> VisualMatcher {
    match matcher {
        VisualMatcher::Text(_) => VisualMatcher::Text(value),
        VisualMatcher::Regex(_) => VisualMatcher::Regex(value),
    }
}

fn color_selection(color: Option<&VisualColor>) -> &str {
    color
        .map(|color| color.0.as_str())
        .filter(|token| VISUAL_COLOR_PALETTE.iter().any(|(value, _)| value == token))
        .unwrap_or("default")
}

fn visual_color_from_selection(value: &str) -> Option<VisualColor> {
    (value != "default"
        && VISUAL_COLOR_PALETTE
            .iter()
            .any(|(token, _)| *token == value))
    .then(|| VisualColor(value.to_string()))
}

fn update_rule_color(rule: &mut ManagedVisualRule, role: VisualColorRole, value: &str) {
    let color = visual_color_from_selection(value);
    match role {
        VisualColorRole::Foreground => rule.style.foreground = color,
        VisualColorRole::Background => rule.style.background = color,
    }
}

#[component]
pub fn VisualRuleEditor(
    rule: ManagedVisualRule,
    save: Callback<ManagedVisualRule>,
    close: Callback<()>,
) -> impl IntoView {
    let (draft, set_draft) = signal(rule);
    let save_draft = move |_| save.run(draft.get());
    view! {
        <div class="visual-rules-modal-backdrop" role="presentation" on:keydown=move |event| {
            if event.key() == "Escape" { close.run(()); }
        }>
            <section class="visual-rules-modal" role="dialog" aria-modal="true" aria-label="Edit visual rule">
                <label>"Name"
                    <input value=move || draft.get().name.unwrap_or_default() on:input=move |event| {
                        let value = event_target_value(&event);
                        set_draft.update(|rule| rule.name = (!value.trim().is_empty()).then_some(value));
                    } />
                </label>
                <label>"Pattern"
                    <input value=move || match draft.get().matcher { VisualMatcher::Text(value) | VisualMatcher::Regex(value) => value } on:input=move |event| {
                        let value = event_target_value(&event);
                        set_draft.update(|rule| rule.matcher = updated_matcher_pattern(rule.matcher.clone(), value));
                    } />
                </label>
                <label><input type="checkbox" checked=move || draft.get().enabled on:change=move |event| set_draft.update(|rule| rule.enabled = event_target_checked(&event)) />"Enabled"</label>
                <label>"Foreground color"
                    <select prop:value=move || color_selection(draft.get().style.foreground.as_ref()).to_string() on:change=move |event| {
                        let value = event_target_value(&event);
                        set_draft.update(|rule| update_rule_color(rule, VisualColorRole::Foreground, &value));
                    }>
                        {VISUAL_COLOR_PALETTE.into_iter().map(|(value, label)| view! {
                            <option value=value>{label}</option>
                        }).collect_view()}
                    </select>
                </label>
                <label>"Background color"
                    <select prop:value=move || color_selection(draft.get().style.background.as_ref()).to_string() on:change=move |event| {
                        let value = event_target_value(&event);
                        set_draft.update(|rule| update_rule_color(rule, VisualColorRole::Background, &value));
                    }>
                        {VISUAL_COLOR_PALETTE.into_iter().map(|(value, label)| view! {
                            <option value=value>{label}</option>
                        }).collect_view()}
                    </select>
                </label>
                <div><button type="button" on:click=save_draft>"Apply to List"</button><button type="button" on:click=move |_| close.run(())>"Cancel"</button></div>
            </section>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regex_rule() -> ManagedVisualRule {
        ManagedVisualRule {
            name: Some("Errors".to_string()),
            enabled: false,
            matcher: VisualMatcher::Regex("^ERROR".to_string()),
            case_sensitive: true,
            style: LineStyleIntent {
                foreground: Some(VisualColor("purple".to_string())),
                background: Some(VisualColor("default".to_string())),
            },
        }
    }

    #[test]
    fn pattern_edit_preserves_matcher_variant() {
        assert_eq!(
            updated_matcher_pattern(VisualMatcher::Regex("old".into()), "new".into()),
            VisualMatcher::Regex("new".into())
        );
        assert_eq!(
            updated_matcher_pattern(VisualMatcher::Text("old".into()), "new".into()),
            VisualMatcher::Text("new".into())
        );
    }

    #[test]
    fn loaded_colors_convert_to_palette_selections_and_default_means_no_color() {
        let rule = regex_rule();

        assert_eq!(color_selection(rule.style.foreground.as_ref()), "purple");
        assert_eq!(color_selection(rule.style.background.as_ref()), "default");
        assert_eq!(visual_color_from_selection("default"), None);
        assert_eq!(
            visual_color_from_selection("cyan"),
            Some(VisualColor("cyan".to_string()))
        );
    }

    #[test]
    fn color_edits_preserve_matcher_variant_and_unrelated_rule_fields() {
        let original = regex_rule();
        let mut edited = original.clone();

        update_rule_color(&mut edited, VisualColorRole::Foreground, "orange");
        update_rule_color(&mut edited, VisualColorRole::Background, "blue");

        assert_eq!(edited.name, original.name);
        assert_eq!(edited.enabled, original.enabled);
        assert_eq!(edited.matcher, original.matcher);
        assert_eq!(edited.case_sensitive, original.case_sensitive);
        assert_eq!(
            edited.style.foreground,
            Some(VisualColor("orange".to_string()))
        );
        assert_eq!(
            edited.style.background,
            Some(VisualColor("blue".to_string()))
        );
    }
}
