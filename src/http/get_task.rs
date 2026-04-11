use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::BabataResult;
use crate::task::{TaskRecord, TaskStatus};

use super::{HttpApp, parse_task_id};

pub(super) async fn handle(
    State(state): State<HttpApp>,
    Path(task_id): Path<String>,
) -> BabataResult<Json<TaskResponse>> {
    let task_id = parse_task_id(&task_id)?;

    let task = state.task_manager.get_task(task_id)?;
    Ok(Json(TaskResponse::from_record(task)))
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct TaskResponse {
    pub(crate) task_id: Uuid,
    pub(crate) description: String,
    pub(crate) agent: String,
    pub(crate) status: TaskStatus,
    pub(crate) parent_task_id: Option<Uuid>,
    pub(crate) root_task_id: Uuid,
    pub(crate) created_at: i64,
    pub(crate) never_ends: bool,
}

impl TaskResponse {
    pub(crate) fn from_record(record: TaskRecord) -> Self {
        Self {
            task_id: record.task_id,
            description: record.description,
            agent: record.agent,
            status: record.status,
            parent_task_id: record.parent_task_id,
            root_task_id: record.root_task_id,
            created_at: record.created_at,
            never_ends: record.never_ends,
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
            "agent": "babata",
            "status": "running",
            "parent_task_id": null,
            "root_task_id": "12345678-1234-1234-1234-123456789abc",
            "created_at": 123,
        }))
        .expect_err("missing never_ends should fail");

        assert!(error.to_string().contains("never_ends"));
    }
}
