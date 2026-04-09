use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};

use crate::{
    BabataResult,
    error::BabataError,
    task::{TaskRecord, TaskStatus},
};

use super::{HttpApp, get_task::TaskResponse};

pub(super) async fn handle(
    State(state): State<HttpApp>,
    Query(query): Query<ListTasksQuery>,
) -> BabataResult<Json<ListTasksResponse>> {
    let status = match query.status {
        Some(status) => match status.parse::<TaskStatus>() {
            Ok(status) => Some(status),
            Err(err) => return Err(BabataError::invalid_input(err)),
        },
        None => None,
    };

    let tasks = state
        .task_manager
        .list_tasks(status, query.limit, query.offset)?;
    Ok(Json(ListTasksResponse::from_records(tasks)))
}

#[derive(Debug, Deserialize)]
pub(super) struct ListTasksQuery {
    #[serde(default)]
    status: Option<String>,
    limit: usize,
    #[serde(default)]
    offset: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ListTasksResponse {
    pub(crate) tasks: Vec<TaskResponse>,
}

impl ListTasksResponse {
    pub(crate) fn from_records(records: Vec<TaskRecord>) -> Self {
        Self {
            tasks: records.into_iter().map(TaskResponse::from_record).collect(),
        }
    }
}
