use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::task::CreateTaskRequest;

use super::{ApiError, HttpApp};

pub(super) async fn handle(
    State(state): State<HttpApp>,
    Json(request): Json<CreateTaskRequest>,
) -> Response {
    if request.prompt.is_empty() {
        return ApiError::bad_request("prompt cannot be empty").into_response();
    }

    match state.task_manager.create_task(CreateTaskRequest {
        prompt: request.prompt,
        parent_task_id: request.parent_task_id,
        agent: request.agent.filter(|value| !value.trim().is_empty()),
        never_ends: request.never_ends,
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

#[derive(Debug, Serialize)]
struct CreateTaskResponse {
    task_id: String,
    status: String,
}
