use axum::{
    Json,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ApiError, HttpApp};

pub(super) async fn pause(State(state): State<HttpApp>, Path(task_id): Path<String>) -> Response {
    control_task(state, &task_id, TaskAction::Pause)
}

pub(super) async fn resume(State(state): State<HttpApp>, Path(task_id): Path<String>) -> Response {
    control_task(state, &task_id, TaskAction::Resume)
}

pub(super) async fn cancel(State(state): State<HttpApp>, Path(task_id): Path<String>) -> Response {
    control_task(state, &task_id, TaskAction::Cancel)
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

    relaunch_task(state, &task_id, reason)
}

fn control_task(state: HttpApp, task_id: &str, action: TaskAction) -> Response {
    let task_id = match parse_task_id(task_id) {
        Ok(task_id) => task_id,
        Err(err) => return err.into_response(),
    };

    let result = match action {
        TaskAction::Pause => state.task_manager.pause_task(task_id),
        TaskAction::Resume => state.task_manager.resume_task(task_id),
        TaskAction::Cancel => state.task_manager.cancel_task(task_id),
        TaskAction::Relaunch => {
            return ApiError::bad_request("relaunch requires a reason").into_response();
        }
    };
    if let Err(err) = result {
        return ApiError::from_babata_error(err).into_response();
    }

    Json(ActionResponse {
        ok: true,
        task_id: task_id.to_string(),
        action: action.as_str().to_string(),
    })
    .into_response()
}

fn relaunch_task(state: HttpApp, task_id: &str, reason: &str) -> Response {
    let task_id = match parse_task_id(task_id) {
        Ok(task_id) => task_id,
        Err(err) => return err.into_response(),
    };

    if let Err(err) = state.task_manager.relaunch_task(task_id, reason) {
        return ApiError::from_babata_error(err).into_response();
    }

    Json(ActionResponse {
        ok: true,
        task_id: task_id.to_string(),
        action: TaskAction::Relaunch.as_str().to_string(),
    })
    .into_response()
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
    Relaunch,
}

impl TaskAction {
    const fn as_str(self) -> &'static str {
        match self {
            TaskAction::Pause => "pause",
            TaskAction::Resume => "resume",
            TaskAction::Cancel => "cancel",
            TaskAction::Relaunch => "relaunch",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RelaunchTaskRequest {
    pub(crate) reason: String,
}

#[derive(Debug, Serialize)]
struct ActionResponse {
    ok: bool,
    task_id: String,
    action: String,
}
