use crate::api::commons::{ApiError, VisualRulesResponse, VisualRulesSaveRequest};
use crate::api::config::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use logmancer_core::{SaveOutcome, SaveResult, VisualRulesEnvelope, VisualRulesError};

#[derive(Clone, Copy)]
enum VisualRulesApiAction {
    Save,
    Reload,
}

impl VisualRulesApiAction {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Save => "save",
            Self::Reload => "reload",
        }
    }
}

pub async fn get_visual_rules(State(app_state): State<AppState>) -> impl IntoResponse {
    let state = app_state.registry.visual_rules_state();
    (
        StatusCode::OK,
        Json(VisualRulesResponse {
            revision: state.revision,
            envelope: state.envelope,
            diagnostics: state
                .diagnostics
                .into_iter()
                .map(|diagnostic| diagnostic.message)
                .collect(),
        }),
    )
}

pub async fn save_visual_rules(
    State(app_state): State<AppState>,
    Json(request): Json<VisualRulesSaveRequest>,
) -> Response {
    let envelope = request.envelope.clone();
    match app_state
        .registry
        .upsert_visual_rules(request.base_revision, request.envelope)
    {
        Ok(result) => {
            if let Some(io_warning) = result.io_warning() {
                log_visual_rules_io(VisualRulesApiAction::Save, io_warning);
            }
            (StatusCode::OK, Json(visual_rules_success(result, envelope))).into_response()
        }
        Err(error) => visual_rules_error(VisualRulesApiAction::Save, error),
    }
}

pub async fn retry_visual_rules(State(app_state): State<AppState>) -> Response {
    match app_state.registry.reload_visual_rules() {
        Ok(state) => (
            StatusCode::OK,
            Json(VisualRulesResponse {
                revision: state.revision,
                envelope: state.envelope,
                diagnostics: state
                    .diagnostics
                    .into_iter()
                    .map(|diagnostic| diagnostic.message)
                    .collect(),
            }),
        )
            .into_response(),
        Err(error) => visual_rules_error(VisualRulesApiAction::Reload, error),
    }
}

fn visual_rules_success(result: SaveResult, envelope: VisualRulesEnvelope) -> VisualRulesResponse {
    VisualRulesResponse {
        revision: result.revision,
        envelope,
        diagnostics: match result.outcome {
            SaveOutcome::Committed => Vec::new(),
            SaveOutcome::CommittedWithWarning(message) => vec![message],
        },
    }
}

fn visual_rules_error(action: VisualRulesApiAction, error: VisualRulesError) -> Response {
    if let Some(io_error) = error.io_error() {
        log_visual_rules_io(action, io_error);
    }
    let status = match &error {
        VisualRulesError::Validation(_) | VisualRulesError::Decode(_) => {
            StatusCode::UNPROCESSABLE_ENTITY
        }
        VisualRulesError::RevisionConflict | VisualRulesError::SourceConflict => {
            StatusCode::CONFLICT
        }
        VisualRulesError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    let message = if error.is_io() {
        format!(
            "Could not {} visual rules. Check the application logs for details.",
            action.as_str()
        )
    } else {
        error.to_string()
    };
    (
        status,
        Json(ApiError {
            code: "visual_rules_error".to_string(),
            message,
        }),
    )
        .into_response()
}

fn log_visual_rules_io(action: VisualRulesApiAction, error: &logmancer_core::VisualRulesIoError) {
    tracing::error!(
        api_action = action.as_str(),
        persistence_stage = error.stage().as_str(),
        path = %error.path().display(),
        error_kind = ?error.kind(),
        raw_os_error = ?error.raw_os_error(),
        error_chain = %error.causal_chain(),
        "visual rules persistence failed"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use logmancer_core::{VisualRulesIoError, VisualRulesPersistenceStage};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use tracing::field::{Field, Visit};
    use tracing_subscriber::layer::{Context, Layer};
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::registry::LookupSpan;

    #[derive(Clone, Default)]
    struct EventCollector(Arc<Mutex<Vec<HashMap<String, String>>>>);

    struct FieldVisitor<'a>(&'a mut HashMap<String, String>);

    impl Visit for FieldVisitor<'_> {
        fn record_str(&mut self, field: &Field, value: &str) {
            self.0.insert(field.name().to_string(), value.to_string());
        }

        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            self.0
                .insert(field.name().to_string(), format!("{value:?}"));
        }
    }

    impl<S> Layer<S> for EventCollector
    where
        S: tracing::Subscriber + for<'lookup> LookupSpan<'lookup>,
    {
        fn on_event(&self, event: &tracing::Event<'_>, _context: Context<'_, S>) {
            let mut fields = HashMap::new();
            fields.insert("level".to_string(), event.metadata().level().to_string());
            event.record(&mut FieldVisitor(&mut fields));
            self.0.lock().expect("event collector lock").push(fields);
        }
    }

    #[test]
    fn success_payload_uses_the_completed_operation_snapshot() {
        let envelope = VisualRulesEnvelope::new(Vec::new());
        let result = logmancer_core::VisualRulesManager::in_memory()
            .apply_memory(envelope.clone())
            .expect("apply visual rules in memory");
        let response = visual_rules_success(result, envelope.clone());
        assert_eq!(response.revision, 1);
        assert_eq!(response.envelope, envelope);
    }

    #[tokio::test]
    async fn io_failure_returns_safe_actionable_response_and_emits_structured_error() {
        let collector = EventCollector::default();
        let captured = collector.0.clone();
        let subscriber = tracing_subscriber::registry().with(collector);
        let sensitive_path = std::path::PathBuf::from("/private/user/visual-rules.json");
        let error = VisualRulesError::Io(VisualRulesIoError::new(
            VisualRulesPersistenceStage::AtomicCommit,
            sensitive_path.clone(),
            std::io::Error::from_raw_os_error(5),
        ));

        let response = tracing::subscriber::with_default(subscriber, || {
            visual_rules_error(VisualRulesApiAction::Save, error)
        });
        let status = response.status();
        let body = String::from_utf8(
            to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("read response")
                .to_vec(),
        )
        .expect("UTF-8 response");

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(body.contains("Could not save visual rules"));
        assert!(!body.contains(&sensitive_path.display().to_string()));
        assert!(!body.contains("os error 5"));

        let events = captured.lock().expect("captured events lock");
        let event = events.last().expect("persistence error event");
        assert_eq!(event.get("level").map(String::as_str), Some("ERROR"));
        assert_eq!(event.get("api_action").map(String::as_str), Some("save"));
        assert_eq!(
            event.get("persistence_stage").map(String::as_str),
            Some("atomic_commit")
        );
        assert_eq!(
            event.get("path").map(String::as_str),
            Some(sensitive_path.to_string_lossy().as_ref())
        );
        assert!(event.contains_key("error_kind"));
        assert_eq!(
            event.get("raw_os_error").map(String::as_str),
            Some("Some(5)")
        );
        assert!(event["error_chain"].contains("os error 5"));
    }

    #[test]
    fn reload_io_failure_identifies_the_api_action() {
        let collector = EventCollector::default();
        let captured = collector.0.clone();
        let subscriber = tracing_subscriber::registry().with(collector);

        let response = tracing::subscriber::with_default(subscriber, || {
            visual_rules_error(
                VisualRulesApiAction::Reload,
                VisualRulesError::Io(VisualRulesIoError::new(
                    VisualRulesPersistenceStage::SourceOpen,
                    "/private/user/visual-rules.json",
                    std::io::Error::from_raw_os_error(1),
                )),
            )
        });

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let events = captured.lock().expect("captured events lock");
        assert_eq!(
            events.last().and_then(|event| event.get("api_action")),
            Some(&"reload".to_string())
        );
    }

    #[test]
    fn validation_and_conflicts_do_not_emit_error_events() {
        let collector = EventCollector::default();
        let captured = collector.0.clone();
        let subscriber = tracing_subscriber::registry().with(collector);

        tracing::subscriber::with_default(subscriber, || {
            let validation = visual_rules_error(
                VisualRulesApiAction::Save,
                VisualRulesError::Validation("invalid rule".to_string()),
            );
            let conflict = visual_rules_error(
                VisualRulesApiAction::Reload,
                VisualRulesError::SourceConflict,
            );
            assert_eq!(validation.status(), StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(conflict.status(), StatusCode::CONFLICT);
        });

        assert!(captured.lock().expect("captured events lock").is_empty());
    }
}
