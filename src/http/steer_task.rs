use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use crate::{
    http::{ApiError, HttpApp},
    message::Content,
};

pub(super) async fn handle(
    State(state): State<HttpApp>,
    Path(task_id): Path<Uuid>,
    Json(request): Json<SteerTaskRequest>,
) -> Response {
    if request.content.is_empty() {
        return ApiError::bad_request("content cannot be empty").into_response();
    }

    match state
        .task_manager
        .steer_task(task_id, request.content)
        .await
    {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({ "success": true }))).into_response(),
        Err(err) => ApiError::from_babata_error(err).into_response(),
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct SteerTaskRequest {
    pub content: Vec<Content>,
}
