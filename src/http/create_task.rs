use axum::{Json, extract::State};
use serde::Serialize;

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

    let task_id = state.task_manager.create_task(request)?;
    Ok(Json(CreateTaskResponse {
        task_id: task_id.to_string(),
        status: TaskStatus::Running.to_string(),
    }))
}

#[derive(Debug, Serialize)]
pub(crate) struct CreateTaskResponse {
    task_id: String,
    status: String,
}
