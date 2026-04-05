use axum::{
    Json,
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::utils::babata_dir;

use super::{ApiError, HttpApp};

const MAX_LIMIT: usize = 1000;

#[derive(Debug, Deserialize)]
pub(crate) struct LogQueryParams {
    /// Required: Maximum number of log lines to return (1-1000)
    limit: usize,
    /// Optional: Number of lines to skip (default: 0)
    #[serde(default)]
    offset: usize,
}

pub(super) async fn handle(
    State(state): State<HttpApp>,
    Path(task_id): Path<String>,
    Query(params): Query<LogQueryParams>,
) -> Response {
    // Parse and validate task ID
    let task_id = match Uuid::parse_str(&task_id) {
        Ok(task_id) => task_id,
        Err(err) => {
            return ApiError::bad_request(format!("Invalid task id '{}': {}", task_id, err))
                .into_response();
        }
    };

    // Verify task exists
    if let Err(err) = state.task_manager.get_task(task_id) {
        return ApiError::from_babata_error(err).into_response();
    }

    let task_id_str = task_id.to_string();

    // Validate limit: must be greater than 0 and not exceed MAX_LIMIT
    if params.limit == 0 {
        return ApiError::bad_request("limit must be greater than 0").into_response();
    }
    if params.limit > MAX_LIMIT {
        return ApiError::bad_request(format!(
            "limit exceeds maximum value of {}",
            MAX_LIMIT
        ))
        .into_response();
    }

    let limit = params.limit;
    let offset = params.offset;

    // Read and filter logs
    match read_task_logs(&task_id_str, offset, limit).await {
        Ok(logs) => Json(logs).into_response(),
        Err(err) => ApiError::bad_request(format!("Failed to read logs: {}", err)).into_response(),
    }
}

/// Read logs from all log files in the log directory and filter by task_id with pagination
async fn read_task_logs(
    task_id: &str,
    offset: usize,
    limit: usize,
) -> Result<Vec<String>, std::io::Error> {
    let log_dir = babata_dir()
        .map_err(|e| std::io::Error::other(e.to_string()))?
        .join("logs");

    // Collect all log lines from all .log files in the log directory
    let mut all_matching_lines: Vec<String> = Vec::new();

    // Read directory entries
    let mut entries = tokio::fs::read_dir(&log_dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        // Only process .log files
        if path.extension().map(|e| e == "log").unwrap_or(false) && path.is_file() {
            // Read the log file
            match tokio::fs::read_to_string(&path).await {
                Ok(content) => {
                    // Filter lines containing the task_id
                    let lines: Vec<String> = content
                        .lines()
                        .filter(|line| line.contains(task_id))
                        .map(|line| line.to_string())
                        .collect();
                    all_matching_lines.extend(lines);
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    // File was removed between listing and reading, skip
                    continue;
                }
                Err(e) => {
                    log::warn!("Failed to read log file {:?}: {}", path, e);
                    continue;
                }
            };
        }
    }

    // Sort lines by timestamp if possible (lines typically start with timestamp)
    all_matching_lines.sort();

    // Apply pagination
    let paginated_logs: Vec<String> = all_matching_lines
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();

    Ok(paginated_logs)
}
