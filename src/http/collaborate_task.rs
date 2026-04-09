use axum::{
    Json,
    extract::{Path, State},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{BabataResult, error::BabataError, task::CollaborationTaskState};

use super::HttpApp;

pub(super) async fn create(
    State(state): State<HttpApp>,
    Path(task_id): Path<Uuid>,
    Json(request): Json<CollaborateTaskRequest>,
) -> BabataResult<()> {
    if request.agent.trim().is_empty() {
        return Err(BabataError::invalid_input("agent cannot be empty"));
    }
    if request.prompt.trim().is_empty() {
        return Err(BabataError::invalid_input("prompt cannot be empty"));
    }

    state.task_manager.collaborate_task(task_id, request)?;
    Ok(())
}

pub(super) async fn get(
    State(state): State<HttpApp>,
    Path(task_id): Path<Uuid>,
) -> BabataResult<Json<CollaborationTaskState>> {
    let collaboration_state = state.task_manager.get_collaboration_task_state(task_id)?;
    Ok(Json(collaboration_state))
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
