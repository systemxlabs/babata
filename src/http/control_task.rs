use axum::{
    Json,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
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

pub(super) async fn relaunch(
    State(state): State<HttpApp>,
    Path(task_id): Path<String>,
    Json(request): Json<RelaunchTaskRequest>,
) -> Response {
    let reason = request.reason.trim();
    if reason.is_empty() {
        return ApiError::bad_request("reason cannot be empty").into_response();
    }

    relaunch_task(state, &task_id, reason).into_response()
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

fn relaunch_task(state: HttpApp, task_id: &str, reason: &str) -> Result<(), ApiError> {
    let task_id = parse_task_id(task_id)?;
    state
        .task_manager
        .relaunch_task(task_id, reason)
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

#[derive(Debug, Clone, Copy)]
enum TaskAction {
    Pause,
    Resume,
    Cancel,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RelaunchTaskRequest {
    pub(crate) reason: String,
}
