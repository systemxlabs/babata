use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    BabataResult,
    task::{TaskRecord, TaskStatus},
};

use super::HttpApp;

pub(super) async fn handle(
    State(state): State<HttpApp>,
    Query(query): Query<ListRootTasksQuery>,
) -> BabataResult<Json<ListRootTasksResponse>> {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);
    let offset = (page.saturating_sub(1)) * page_size;

    let (tasks, total) = state
        .task_manager
        .list_root_tasks(query.status, page_size, offset)?;

    Ok(Json(ListRootTasksResponse {
        tasks: tasks
            .into_iter()
            .map(RootTaskResponse::from_record)
            .collect(),
        total,
        page,
        page_size,
    }))
}

#[derive(Debug, Deserialize)]
pub(super) struct ListRootTasksQuery {
    #[serde(default)]
    status: Option<TaskStatus>,
    #[serde(default)]
    page: Option<usize>,
    #[serde(default)]
    page_size: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ListRootTasksResponse {
    pub(crate) tasks: Vec<RootTaskResponse>,
    pub(crate) total: usize,
    pub(crate) page: usize,
    pub(crate) page_size: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RootTaskResponse {
    pub(crate) task_id: Uuid,
    pub(crate) description: String,
    pub(crate) agent: String,
    pub(crate) status: TaskStatus,
    pub(crate) parent_task_id: Option<Uuid>,
    pub(crate) root_task_id: Uuid,
    pub(crate) created_at: i64,
    pub(crate) never_ends: bool,
    pub(crate) subtask_count: usize,
}

impl RootTaskResponse {
    pub(crate) fn from_record((record, subtask_count): (TaskRecord, usize)) -> Self {
        Self {
            task_id: record.task_id,
            description: record.description,
            agent: record.agent,
            status: record.status,
            parent_task_id: record.parent_task_id,
            root_task_id: record.root_task_id,
            created_at: record.created_at,
            never_ends: record.never_ends,
            subtask_count,
        }
    }
}
