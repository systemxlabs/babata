use std::fs;

use axum::{
    Json,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use serde::Serialize;
use uuid::Uuid;

use crate::task::task_dir;

use super::{ApiError, HttpApp};

pub(super) async fn handle(State(state): State<HttpApp>, Path(task_id): Path<String>) -> Response {
    let task_id = match parse_task_id(&task_id) {
        Ok(task_id) => task_id,
        Err(err) => return err.into_response(),
    };

    if let Err(err) = state.task_manager.get_task(task_id) {
        return ApiError::from_babata_error(err).into_response();
    }

    let task_dir = match task_dir(task_id) {
        Ok(task_dir) => task_dir,
        Err(err) => return ApiError::from_babata_error(err).into_response(),
    };

    let task_markdown = match read_markdown(&task_dir.join("task.md"), "task.md") {
        Ok(task_markdown) => task_markdown,
        Err(err) => return err.into_response(),
    };
    let progress_markdown = match read_markdown(&task_dir.join("progress.md"), "progress.md") {
        Ok(progress_markdown) => progress_markdown,
        Err(err) => return err.into_response(),
    };

    Json(TaskContentResponse {
        task_id: task_id.to_string(),
        task_markdown,
        progress_markdown,
    })
    .into_response()
}

fn parse_task_id(task_id: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(task_id)
        .map_err(|err| ApiError::bad_request(format!("Invalid task id '{}': {}", task_id, err)))
}

fn read_markdown(path: &std::path::Path, label: &str) -> Result<String, ApiError> {
    fs::read_to_string(path).map_err(|err| {
        ApiError::bad_request(format!(
            "Failed to read {label} '{}': {}",
            path.display(),
            err
        ))
    })
}

#[derive(Debug, Serialize)]
struct TaskContentResponse {
    task_id: String,
    task_markdown: String,
    progress_markdown: String,
}
