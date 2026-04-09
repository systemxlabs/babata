use axum::{
    Json,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ApiError, HttpApp};

pub(super) async fn handle(
    State(state): State<HttpApp>,
    Path(task_id): Path<String>,
    Json(request): Json<ControlTaskRequest>,
) -> Response {
    control_task(state, &task_id, request.action).into_response()
}

fn control_task(state: HttpApp, task_id: &str, action: TaskAction) -> Result<(), ApiError> {
    let task_id = parse_task_id(task_id)?;

    match action {
        TaskAction::Pause => state.task_manager.pause_task(task_id),
        TaskAction::Resume => state.task_manager.resume_task(task_id),
        TaskAction::Cancel => state.task_manager.cancel_task(task_id),
    }
    .map_err(ApiError::from_babata_error)?;

    Ok(())
}

fn parse_task_id(task_id: &str) -> Result<Uuid, ApiError> {
    let task_id = match Uuid::parse_str(task_id) {
        Ok(task_id) => task_id,
        Err(err) => {
            return Err(ApiError::bad_request(format!(
                "Invalid task id '{}': {}",
                task_id, err
            )));
        }
    };
    Ok(task_id)
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum TaskAction {
    Pause,
    Resume,
    Cancel,
}

impl std::fmt::Display for TaskAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            TaskAction::Pause => "pause",
            TaskAction::Resume => "resume",
            TaskAction::Cancel => "cancel",
        };
        f.write_str(value)
    }
}

impl std::str::FromStr for TaskAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pause" => Ok(TaskAction::Pause),
            "resume" => Ok(TaskAction::Resume),
            "cancel" => Ok(TaskAction::Cancel),
            _ => Err(format!(
                "Unsupported task action '{}'; expected pause, resume, or cancel",
                s
            )),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ControlTaskRequest {
    pub(crate) action: TaskAction,
}

#[cfg(test)]
mod tests {
    use super::{ControlTaskRequest, TaskAction};
    use serde_json::json;

    #[test]
    fn control_task_request_deserializes_action() {
        let request = serde_json::from_value::<ControlTaskRequest>(json!({
            "action": "pause"
        }))
        .expect("deserialize request");

        assert!(matches!(request.action, TaskAction::Pause));
    }

    #[test]
    fn control_task_request_rejects_unknown_fields() {
        let error = serde_json::from_value::<ControlTaskRequest>(json!({
            "action": "pause",
            "extra": true
        }))
        .expect_err("unknown field should fail");

        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn task_action_from_str_parses_supported_values() {
        assert!(matches!(
            "pause".parse::<TaskAction>(),
            Ok(TaskAction::Pause)
        ));
        assert!(matches!(
            "resume".parse::<TaskAction>(),
            Ok(TaskAction::Resume)
        ));
        assert!(matches!(
            "cancel".parse::<TaskAction>(),
            Ok(TaskAction::Cancel)
        ));
    }
}
