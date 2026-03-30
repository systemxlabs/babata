use axum::{
    Json,
    extract::{Query, State},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::task::{TaskListQuery, TaskRecord, TaskStatus};

use super::{ApiError, HttpApp, get_task::TaskResponse};

pub(super) async fn handle_api(
    State(state): State<HttpApp>,
    Query(query): Query<ApiListTasksQuery>,
) -> Response {
    let status = match parse_status(query.status) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };
    let root_task_id = match parse_task_id(query.root_task_id) {
        Ok(root_task_id) => root_task_id,
        Err(err) => return err.into_response(),
    };

    let query = TaskListQuery {
        status,
        root_only: query.root_only,
        root_task_id,
        limit: query.limit,
        offset: query.offset,
    };

    match state.task_manager.list_tasks_filtered(&query) {
        Ok(tasks) => Json(ListTasksResponse::from_records(tasks)).into_response(),
        Err(err) => ApiError::from_babata_error(err).into_response(),
    }
}

pub(super) async fn handle(
    State(state): State<HttpApp>,
    Query(query): Query<ListTasksQuery>,
) -> Response {
    let status = match query.status {
        Some(status) => match status.parse::<TaskStatus>() {
            Ok(status) => Some(status),
            Err(err) => return ApiError::bad_request(err).into_response(),
        },
        None => None,
    };

    match state
        .task_manager
        .list_tasks(status, query.limit, query.offset)
    {
        Ok(tasks) => Json(ListTasksResponse::from_records(tasks)).into_response(),
        Err(err) => ApiError::from_babata_error(err).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct ListTasksQuery {
    #[serde(default)]
    status: Option<String>,
    limit: usize,
    #[serde(default)]
    offset: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ApiListTasksQuery {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    root_only: bool,
    #[serde(default)]
    root_task_id: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    offset: Option<usize>,
}

fn default_limit() -> usize {
    100
}

fn parse_status(status: Option<String>) -> Result<Option<TaskStatus>, ApiError> {
    match status {
        Some(status) => status
            .parse::<TaskStatus>()
            .map(Some)
            .map_err(ApiError::bad_request),
        None => Ok(None),
    }
}

fn parse_task_id(task_id: Option<String>) -> Result<Option<Uuid>, ApiError> {
    match task_id {
        Some(task_id) => Uuid::parse_str(&task_id).map(Some).map_err(|err| {
            ApiError::bad_request(format!("Invalid root_task_id '{}': {}", task_id, err))
        }),
        None => Ok(None),
    }
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
