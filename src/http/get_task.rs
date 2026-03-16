use axum::{
    Json,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use serde::Serialize;
use uuid::Uuid;

use crate::task::TaskRecord;

use super::{ApiError, HttpApp};

pub(super) async fn handle(State(state): State<HttpApp>, Path(task_id): Path<String>) -> Response {
    let task_id = match Uuid::parse_str(&task_id) {
        Ok(task_id) => task_id,
        Err(err) => {
            return ApiError::bad_request(format!("Invalid task id '{}': {}", task_id, err))
                .into_response();
        }
    };

    match state.task_manager.get_task(task_id) {
        Ok(task) => Json(TaskResponse::from_record(task)).into_response(),
        Err(err) => ApiError::from_babata_error(err).into_response(),
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct TaskResponse {
    task_id: String,
    prompt: Vec<crate::message::Content>,
    agent: Option<String>,
    status: String,
    parent_task_id: Option<String>,
    root_task_id: String,
    created_at: i64,
}

impl TaskResponse {
    pub(crate) fn from_record(record: TaskRecord) -> Self {
        Self {
            task_id: record.task_id.to_string(),
            prompt: record.prompt,
            agent: record.agent,
            status: record.status.as_str().to_string(),
            parent_task_id: record.parent_task_id.map(|task_id| task_id.to_string()),
            root_task_id: record.root_task_id.to_string(),
            created_at: record.created_at,
        }
    }
}
