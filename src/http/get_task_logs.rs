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

    let entries = match load_log_entries(&task_dir) {
        Ok(entries) => entries,
        Err(err) => return err.into_response(),
    };

    if entries.is_empty() {
        return Json(TaskLogsUnsupportedResponse {
            task_id: task_id.to_string(),
            supported: false,
            reason: "No known log files for this agent".to_string(),
        })
        .into_response();
    }

    Json(TaskLogsResponse {
        task_id: task_id.to_string(),
        supported: true,
        entries,
    })
    .into_response()
}

fn parse_task_id(task_id: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(task_id)
        .map_err(|err| ApiError::bad_request(format!("Invalid task id '{}': {}", task_id, err)))
}

fn load_log_entries(task_dir: &std::path::Path) -> Result<Vec<LogEntryResponse>, ApiError> {
    let candidates = [
        "codex-last-message.md",
        "codex-stdout.log",
        "codex-stderr.log",
        "stdout.log",
        "stderr.log",
    ];
    let mut entries = Vec::new();

    for candidate in candidates {
        let path = task_dir.join(candidate);
        if !path.exists() {
            continue;
        }

        let content = fs::read_to_string(&path).map_err(|err| {
            ApiError::bad_request(format!(
                "Failed to read log file '{}': {}",
                path.display(),
                err
            ))
        })?;

        entries.push(LogEntryResponse {
            path: candidate.to_string(),
            content,
        });
    }

    Ok(entries)
}

#[derive(Debug, Serialize)]
struct TaskLogsResponse {
    task_id: String,
    supported: bool,
    entries: Vec<LogEntryResponse>,
}

#[derive(Debug, Serialize)]
struct TaskLogsUnsupportedResponse {
    task_id: String,
    supported: bool,
    reason: String,
}

#[derive(Debug, Serialize)]
struct LogEntryResponse {
    path: String,
    content: String,
}
