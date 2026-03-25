use axum::{
    Json,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use serde::Serialize;
use uuid::Uuid;

use super::{ApiError, HttpApp, get_task::TaskResponse};

pub(super) async fn handle(State(state): State<HttpApp>, Path(task_id): Path<String>) -> Response {
    let task_id = match parse_task_id(&task_id) {
        Ok(task_id) => task_id,
        Err(err) => return err.into_response(),
    };

    let current = match state.task_manager.get_task(task_id) {
        Ok(task) => task,
        Err(err) => return ApiError::from_babata_error(err).into_response(),
    };

    let tree = match state.task_manager.list_root_tree(current.root_task_id) {
        Ok(tree) => tree,
        Err(err) => return ApiError::from_babata_error(err).into_response(),
    };

    let parent = current.parent_task_id.and_then(|parent_task_id| {
        tree.iter()
            .find(|task| task.task_id == parent_task_id)
            .cloned()
            .map(TaskResponse::from_record)
    });
    let children = tree
        .iter()
        .filter(|task| task.parent_task_id == Some(current.task_id))
        .cloned()
        .map(TaskResponse::from_record)
        .collect();

    Json(TaskTreeResponse {
        root_task_id: current.root_task_id.to_string(),
        parent,
        current: TaskResponse::from_record(current),
        children,
    })
    .into_response()
}

fn parse_task_id(task_id: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(task_id)
        .map_err(|err| ApiError::bad_request(format!("Invalid task id '{}': {}", task_id, err)))
}

#[derive(Debug, Serialize)]
struct TaskTreeResponse {
    root_task_id: String,
    parent: Option<TaskResponse>,
    current: TaskResponse,
    children: Vec<TaskResponse>,
}
