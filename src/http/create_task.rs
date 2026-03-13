use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{message::Content, task::TaskRequest};

use super::{ApiError, HttpApp};

pub(super) async fn handle(
    State(state): State<HttpApp>,
    Json(request): Json<CreateTaskRequest>,
) -> Response {
    if request.prompt.trim().is_empty() {
        return ApiError::bad_request("prompt cannot be empty").into_response();
    }

    match state.task_manager.create_task(TaskRequest {
        prompt: vec![Content::Text {
            text: request.prompt,
        }],
        parent_task_id: match request.parent_task_id {
            Some(parent_task_id) => match Uuid::parse_str(&parent_task_id) {
                Ok(parent_task_id) => Some(parent_task_id),
                Err(err) => {
                    return ApiError::bad_request(format!(
                        "Invalid parent_task_id '{}': {}",
                        parent_task_id, err
                    ))
                    .into_response();
                }
            },
            None => None,
        },
        agent: request.agent.filter(|value| !value.trim().is_empty()),
    }) {
        Ok(task_id) => (
            StatusCode::CREATED,
            Json(CreateTaskResponse {
                task_id: task_id.to_string(),
                status: "running".to_string(),
            }),
        )
            .into_response(),
        Err(err) => ApiError::from_babata_error(err).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct CreateTaskRequest {
    prompt: String,
    #[serde(default)]
    agent: Option<String>,
    #[serde(default)]
    parent_task_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct CreateTaskResponse {
    task_id: String,
    status: String,
}
