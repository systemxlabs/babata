use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use serde::Serialize;
use uuid::Uuid;

use crate::task::TaskRecord;

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

    let root = match build_root_node(current.root_task_id, tree.clone()) {
        Ok(root) => root,
        Err(err) => return err.into_response(),
    };

    Json(TaskTreeResponse {
        root_task_id: current.root_task_id.to_string(),
        parent,
        current: TaskResponse::from_record(current),
        children,
        root,
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
    root: TaskTreeNodeResponse,
}

#[derive(Debug, Serialize)]
struct TaskTreeNodeResponse {
    task: TaskResponse,
    children: Vec<TaskTreeNodeResponse>,
}

fn build_root_node(
    root_task_id: Uuid,
    tree: Vec<TaskRecord>,
) -> Result<TaskTreeNodeResponse, ApiError> {
    let mut by_parent: HashMap<Option<Uuid>, Vec<TaskRecord>> = HashMap::new();
    for task in tree.iter().cloned() {
        by_parent.entry(task.parent_task_id).or_default().push(task);
    }

    let root_task = tree
        .into_iter()
        .find(|task| task.task_id == root_task_id)
        .ok_or_else(|| {
            ApiError::bad_request(format!(
                "Root task '{}' is missing from task tree",
                root_task_id
            ))
        })?;

    Ok(build_node(root_task, &by_parent))
}

fn build_node(
    task: TaskRecord,
    by_parent: &HashMap<Option<Uuid>, Vec<TaskRecord>>,
) -> TaskTreeNodeResponse {
    let children = by_parent
        .get(&Some(task.task_id))
        .map(|children| {
            children
                .iter()
                .cloned()
                .map(|child| build_node(child, by_parent))
                .collect()
        })
        .unwrap_or_default();

    TaskTreeNodeResponse {
        task: TaskResponse::from_record(task),
        children,
    }
}
