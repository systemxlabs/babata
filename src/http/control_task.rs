use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

use super::{ApiError, HttpApp};

pub(super) async fn pause(State(state): State<HttpApp>, Path(task_id): Path<String>) -> Response {
    control_task(state, &task_id, TaskAction::Pause).into_response()
}

pub(super) async fn resume(State(state): State<HttpApp>, Path(task_id): Path<String>) -> Response {
    control_task(state, &task_id, TaskAction::Resume).into_response()
}

pub(super) async fn cancel(State(state): State<HttpApp>, Path(task_id): Path<String>) -> Response {
    control_task(state, &task_id, TaskAction::Cancel).into_response()
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

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
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
