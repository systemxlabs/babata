use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use super::{ApiError, HttpApp};

pub(super) async fn handle(State(state): State<HttpApp>, Path(task_id): Path<String>) -> Response {
    let task_id = match Uuid::parse_str(&task_id) {
        Ok(task_id) => task_id,
        Err(err) => {
            return ApiError::bad_request(format!("Invalid task id '{}': {}", task_id, err))
                .into_response();
        }
    };

    match state.task_manager.delete_task(task_id) {
        Ok(()) => ().into_response(),
        Err(err) => ApiError::from_babata_error(err).into_response(),
    }
}
