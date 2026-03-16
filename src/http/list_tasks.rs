use axum::{
    Json,
    extract::{Query, State},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::task::{TaskRecord, TaskStatus};

use super::{ApiError, HttpApp, get_task::TaskResponse};

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

    match state.task_manager.list_tasks(status, query.limit) {
        Ok(tasks) => Json(ListTasksResponse::from_records(tasks)).into_response(),
        Err(err) => ApiError::from_babata_error(err).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct ListTasksQuery {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
struct ListTasksResponse {
    tasks: Vec<TaskResponse>,
}

impl ListTasksResponse {
    fn from_records(records: Vec<TaskRecord>) -> Self {
        Self {
            tasks: records.into_iter().map(TaskResponse::from_record).collect(),
        }
    }
}
