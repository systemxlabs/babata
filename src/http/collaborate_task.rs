use axum::{
    Json,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ApiError, HttpApp};

pub(super) async fn create(
    State(state): State<HttpApp>,
    Path(task_id): Path<Uuid>,
    Json(request): Json<CollaborateTaskRequest>,
) -> Response {
    if request.agent.trim().is_empty() {
        return ApiError::bad_request("agent cannot be empty").into_response();
    }
    if request.prompt.trim().is_empty() {
        return ApiError::bad_request("prompt cannot be empty").into_response();
    }

    state
        .task_manager
        .collaborate_task(task_id, request)
        .map_err(ApiError::from)
        .into_response()
}

pub(super) async fn get(State(state): State<HttpApp>, Path(task_id): Path<Uuid>) -> Response {
    match state.task_manager.get_collaboration_task_state(task_id) {
        Ok(collaboration_state) => Json(collaboration_state).into_response(),
        Err(err) => ApiError::from(err).into_response(),
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CollaborateTaskRequest {
    pub(crate) agent: String,
    pub(crate) prompt: String,
}

#[cfg(test)]
mod tests {
    use super::CollaborateTaskRequest;
    use serde_json::json;

    #[test]
    fn collaborate_task_request_deserializes_fields() {
        let request = serde_json::from_value::<CollaborateTaskRequest>(json!({
            "agent": "reviewer",
            "prompt": "check edge cases"
        }))
        .expect("deserialize request");

        assert_eq!(request.agent, "reviewer");
        assert_eq!(request.prompt, "check edge cases");
    }

    #[test]
    fn collaborate_task_request_rejects_unknown_fields() {
        let error = serde_json::from_value::<CollaborateTaskRequest>(json!({
            "agent": "reviewer",
            "prompt": "check edge cases",
            "extra": true
        }))
        .expect_err("unknown field should fail");

        assert!(error.to_string().contains("unknown field"));
    }
}
