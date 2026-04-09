use axum::{
    Json,
    extract::{Path, State},
};
use uuid::Uuid;

use crate::{BabataResult, error::BabataError, http::HttpApp, message::Content};

pub(super) async fn handle(
    State(state): State<HttpApp>,
    Path(task_id): Path<Uuid>,
    Json(request): Json<SteerTaskRequest>,
) -> BabataResult<()> {
    if request.content.is_empty() {
        return Err(BabataError::invalid_input("content cannot be empty"));
    }

    state
        .task_manager
        .steer_task(task_id, request.content)
        .await?;
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct SteerTaskRequest {
    pub content: Vec<Content>,
}
