use axum::{
    Json,
    extract::{Path, State},
};
use serde::Serialize;
use uuid::Uuid;

use crate::{BabataResult, task::TaskRecord, task::TaskStatus};

use super::{HttpApp, ensure_task_exists, parse_task_id};

pub(super) async fn handle(
    State(state): State<HttpApp>,
    Path(task_id): Path<String>,
) -> BabataResult<Json<TaskTreeResponse>> {
    let task_id = parse_task_id(&task_id)?;
    ensure_task_exists(&state.task_manager, task_id)?;

    let tree = build_task_tree(&state.task_manager, task_id)?;
    Ok(Json(tree))
}

fn build_task_tree(
    task_manager: &crate::task::TaskManager,
    task_id: uuid::Uuid,
) -> BabataResult<TaskTreeResponse> {
    let task = task_manager.get_task(task_id)?;
    let children_records = task_manager.get_task_children(task_id)?;

    let mut children = Vec::new();
    for child in children_records {
        let child_tree = build_task_tree(task_manager, child.task_id)?;
        children.push(child_tree);
    }

    Ok(TaskTreeResponse::from_record(task, children))
}

#[derive(Debug, Serialize)]
pub(crate) struct TaskTreeResponse {
    pub(crate) task_id: Uuid,
    pub(crate) description: String,
    pub(crate) agent: String,
    pub(crate) status: TaskStatus,
    pub(crate) parent_task_id: Option<Uuid>,
    pub(crate) root_task_id: Uuid,
    pub(crate) created_at: i64,
    pub(crate) never_ends: bool,
    pub(crate) children: Vec<TaskTreeResponse>,
}

impl TaskTreeResponse {
    fn from_record(record: TaskRecord, children: Vec<TaskTreeResponse>) -> Self {
        Self {
            task_id: record.task_id,
            description: record.description,
            agent: record.agent,
            status: record.status,
            parent_task_id: record.parent_task_id,
            root_task_id: record.root_task_id,
            created_at: record.created_at,
            never_ends: record.never_ends,
            children,
        }
    }
}
