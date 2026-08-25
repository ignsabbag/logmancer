use crate::api::commons::{ApiError, VisualRulesResponse, VisualRulesSaveRequest};
use crate::api::config::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use logmancer_core::{SaveOutcome, SaveResult, VisualRulesEnvelope, VisualRulesError};

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
            (StatusCode::OK, Json(visual_rules_success(result, envelope))).into_response()
        }
        Err(error) => visual_rules_error(error),
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
        Err(error) => visual_rules_error(error),
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

fn visual_rules_error(error: VisualRulesError) -> Response {
    let status = match error {
        VisualRulesError::Validation(_) | VisualRulesError::Decode(_) => {
            StatusCode::UNPROCESSABLE_ENTITY
        }
        VisualRulesError::RevisionConflict | VisualRulesError::SourceConflict => {
            StatusCode::CONFLICT
        }
        VisualRulesError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(ApiError {
            code: "visual_rules_error".to_string(),
            message: error.to_string(),
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_payload_uses_the_completed_operation_snapshot() {
        let envelope = VisualRulesEnvelope::new(Vec::new());
        let response = visual_rules_success(
            SaveResult {
                revision: 7,
                outcome: SaveOutcome::Committed,
            },
            envelope.clone(),
        );
        assert_eq!(response.revision, 7);
        assert_eq!(response.envelope, envelope);
    }
}
