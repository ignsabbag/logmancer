#[cfg(target_arch = "wasm32")]
use crate::browser_api_client::{fetch_visual_rules, retry_visual_rules, save_visual_rules};
use crate::components::visual_rule_editor::{new_rule, VisualRuleEditor};
#[cfg(target_arch = "wasm32")]
use crate::visual_rules_state::operation_status;
use crate::visual_rules_state::VisualRulesEditorState;
use leptos::html;
use leptos::prelude::*;
use logmancer_core::{ManagedVisualRule, VisualRulesEnvelope};

fn drawer_should_handle_escape(key: &str) -> bool {
    key == "Escape"
}

#[cfg(any(target_arch = "wasm32", test))]
fn notify_after_accepted_save(accepted: bool, notify: impl FnOnce()) {
    if accepted {
        notify();
    }
}

#[component]
pub fn VisualRules(
    open: ReadSignal<bool>,
    set_open: WriteSignal<bool>,
    invoker_ref: NodeRef<html::Button>,
    on_saved: Callback<()>,
) -> impl IntoView {
    #[cfg(not(target_arch = "wasm32"))]
    let _ = on_saved;
    let (editor, set_editor) = signal(None::<usize>);
    let (state, set_state) = signal(VisualRulesEditorState::new(
        0,
        VisualRulesEnvelope::new(Vec::new()),
    ));

    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        if open.get() {
            set_state.update(VisualRulesEditorState::open_drawer);
            leptos::task::spawn_local(async move {
                match fetch_visual_rules().await {
                    Ok(response) => set_state.update(|state| {
                        let message =
                            operation_status("Loaded visual rules.", &response.diagnostics);
                        if state.load_saved_once(response.revision, response.envelope) {
                            state.save_failed(message);
                        }
                    }),
                    Err(error) => set_state.update(|state| state.save_failed(error)),
                }
            });
        }
    });

    let discard = move |_| set_state.update(VisualRulesEditorState::discard);
    let persist = move |_replace: bool| {
        #[cfg(target_arch = "wasm32")]
        {
            let current = state.get();
            if !_replace && !current.ordinary_save_allowed() {
                set_state.update(|state| {
                    state.save_failed(
                    "Reload preserved a conflicting draft. Use Replace or Discard before saving.",
                )
                });
                return;
            }
            let operation = set_state
                .try_update(VisualRulesEditorState::begin_operation)
                .expect("visual rules state");
            leptos::task::spawn_local(async move {
                match save_visual_rules(current.revision(), current.envelope().clone(), _replace)
                    .await
                {
                    Ok(response) => set_state.update(|state| {
                        let action = if _replace { "Replaced" } else { "Saved" };
                        let message = operation_status(
                            &format!("{action} visual rules."),
                            &response.diagnostics,
                        );
                        let accepted = state.save_succeeded_for(
                            operation,
                            response.revision,
                            response.envelope,
                            message,
                        );
                        notify_after_accepted_save(accepted, || on_saved.run(()));
                    }),
                    Err(error) => set_state.update(|state| state.save_failed(error)),
                }
            });
        }
    };
    let reload = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let operation = set_state
                .try_update(VisualRulesEditorState::begin_operation)
                .expect("visual rules state");
            leptos::task::spawn_local(async move {
                match retry_visual_rules().await {
                    Ok(response) => set_state.update(|state| {
                        let message =
                            operation_status("Loaded visual rules.", &response.diagnostics);
                        state.reload_saved_for(
                            operation,
                            response.revision,
                            response.envelope,
                            message,
                        );
                    }),
                    Err(error) => set_state.update(|state| state.save_failed(error)),
                }
            });
        }
    };
    let apply_editor = move |rule: ManagedVisualRule| {
        set_state.update(|state| match editor.get_untracked() {
            Some(index) if index < state.envelope().rules.len() => state.replace_rule(index, rule),
            _ => state.add(rule),
        });
        set_state.update(VisualRulesEditorState::close_editor_with_escape);
        set_editor.set(None);
    };
    view! {
        <div data-viewer-shortcuts="ignore" style="display: contents">
            <aside class=move || if open.get() { "visual-rules-drawer" } else { "visual-rules-drawer visual-rules-drawer--closed" } aria-label="Visual rules" on:keydown=move |event: leptos::ev::KeyboardEvent| {
                if drawer_should_handle_escape(&event.key()) {
                    event.prevent_default();
                    set_state.update(VisualRulesEditorState::collapse);
                    set_state.update(|state| {
                        let _ = state.close_drawer_with_escape();
                    });
                    set_open.set(false);
                    if let Some(invoker) = invoker_ref.get() {
                        request_animation_frame(move || {
                            _ = invoker.focus();
                        });
                    }
                }
            }>
                <header><h2>"Visual Rules"</h2><button type="button" on:click=move |_| {
                    set_state.update(VisualRulesEditorState::collapse);
                    set_state.update(|state| {
                        let _ = state.close_drawer_with_escape();
                    });
                    set_open.set(false);
                    if let Some(invoker) = invoker_ref.get() {
                        request_animation_frame(move || {
                            _ = invoker.focus();
                        });
                    }
                }>"Close"</button></header>
                <p role="status">{move || state.get().status().to_string()}</p>
                <button type="button" on:click=move |_| {
                    let index = state.get_untracked().envelope().rules.len();
                    set_state.update(|state| state.add(new_rule()));
                    set_state.update(|state| state.open_editor(index));
                    set_editor.set(Some(index));
                }>"Add"</button>
                <ol>{move || state.get().envelope().rules.clone().into_iter().enumerate().map(|(index, rule)| view! {
                    <li><button type="button" on:click=move |_| {
                        set_state.update(|state| state.open_editor(index));
                        set_editor.set(Some(index));
                    }>{rule.name.unwrap_or_else(|| "Unnamed rule".to_string())}</button>
                        <button type="button" on:click=move |_| set_state.update(|state| state.move_rule(index, -1))>"Move up"</button>
                        <button type="button" on:click=move |_| set_state.update(|state| state.move_rule(index, 1))>"Move down"</button>
                        <button type="button" on:click=move |_| set_state.update(|state| state.remove(index))>"Remove"</button></li>
                }).collect_view()}</ol>
                <footer><button type="button" on:click=discard>"Discard"</button><button type="button" on:click=reload>"Reload latest"</button><button type="button" disabled=move || !state.get().ordinary_save_allowed() on:click=move |_| persist(false)>"Save"</button><button type="button" on:click=move |_| persist(true)>"Replace"</button></footer>
            </aside>
            {move || editor.get().map(|index| {
                let rule = state.get().envelope().rules.get(index).cloned().unwrap_or_else(new_rule);
                view! { <VisualRuleEditor rule save=Callback::new(apply_editor) close=Callback::new(move |_| {
                    set_state.update(VisualRulesEditorState::close_editor_with_escape);
                    set_editor.set(None);
                }) /> }
            })}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::visual_rules_state::VisualRulesFocusRequest;

    #[test]
    fn drawer_escape_is_handled_locally_and_restores_the_invoker_focus() {
        assert!(drawer_should_handle_escape("Escape"));
        assert!(!drawer_should_handle_escape("ArrowDown"));

        let mut state = VisualRulesEditorState::new(0, VisualRulesEnvelope::new(Vec::new()));
        state.open_drawer();

        assert_eq!(
            state.close_drawer_with_escape(),
            VisualRulesFocusRequest::Invoker
        );
    }

    #[test]
    fn page_refresh_is_notified_only_for_an_accepted_save_response() {
        let mut notifications = 0;

        notify_after_accepted_save(false, || notifications += 1);
        assert_eq!(notifications, 0);

        notify_after_accepted_save(true, || notifications += 1);
        assert_eq!(notifications, 1);
    }
}
