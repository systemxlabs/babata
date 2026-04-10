use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    BabataResult,
    error::BabataError,
    task::{CreateTaskRequest, TaskStatus},
};

use super::HttpApp;

pub(super) async fn handle(
    State(state): State<HttpApp>,
    Json(request): Json<CreateTaskRequest>,
) -> BabataResult<Json<CreateTaskResponse>> {
    if request.prompt.is_empty() {
        return Err(BabataError::invalid_input("prompt cannot be empty"));
    }
    if request.description.trim().is_empty() {
        return Err(BabataError::invalid_input("description cannot be empty"));
    }
    if request.agent.trim().is_empty() {
        return Err(BabataError::invalid_input("agent cannot be empty"));
    }

    let task_id = state.task_manager.create_task(request)?;
    Ok(Json(CreateTaskResponse {
        task_id,
        status: TaskStatus::Running,
    }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CreateTaskResponse {
    pub task_id: Uuid,
    pub status: TaskStatus,
}
