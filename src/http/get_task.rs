use axum::{
    Json,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::task::{TaskRecord, TaskStatus};

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

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct TaskResponse {
    pub(crate) task_id: String,
    pub(crate) description: String,
    pub(crate) agent: Option<String>,
    pub(crate) status: String,
    pub(crate) actions: TaskActions,
    pub(crate) parent_task_id: Option<String>,
    pub(crate) root_task_id: String,
    pub(crate) created_at: i64,
    pub(crate) never_ends: bool,
}

impl TaskResponse {
    pub(crate) fn from_record(record: TaskRecord) -> Self {
        let actions = TaskActions::for_status(record.status);
        Self {
            task_id: record.task_id.to_string(),
            description: record.description,
            agent: record.agent,
            status: record.status.as_str().to_string(),
            actions,
            parent_task_id: record.parent_task_id.map(|task_id| task_id.to_string()),
            root_task_id: record.root_task_id.to_string(),
            created_at: record.created_at,
            never_ends: record.never_ends,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct TaskActions {
    pub(crate) pause: bool,
    pub(crate) resume: bool,
    pub(crate) cancel: bool,
    pub(crate) relaunch: bool,
}

impl TaskActions {
    fn for_status(status: TaskStatus) -> Self {
        Self {
            pause: status == TaskStatus::Running,
            resume: status == TaskStatus::Paused,
            cancel: !matches!(status, TaskStatus::Done | TaskStatus::Canceled),
            relaunch: status == TaskStatus::Running,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TaskResponse;
    use serde_json::json;

    #[test]
    fn task_response_requires_never_ends_when_deserializing() {
        let error = serde_json::from_value::<TaskResponse>(json!({
            "task_id": "12345678-1234-1234-1234-123456789abc",
            "description": "demo",
            "agent": "codex",
            "status": "running",
            "actions": { "pause": true, "resume": false, "cancel": true, "relaunch": true },
            "parent_task_id": null,
            "root_task_id": "12345678-1234-1234-1234-123456789abc",
            "created_at": 123,
        }))
        .expect_err("missing never_ends should fail");

        assert!(error.to_string().contains("never_ends"));
    }
}
