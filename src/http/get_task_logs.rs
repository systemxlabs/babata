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
        return ApiError::bad_request(format!("limit exceeds maximum value of {}", MAX_LIMIT))
            .into_response();
    }

    let limit = params.limit;
    let offset = params.offset;

    // Read and filter logs with pagination
    match read_task_logs(&task_id_str, offset, limit).await {
        Ok(logs) => Json(logs).into_response(),
        Err(err) => ApiError::bad_request(format!("Failed to read logs: {}", err)).into_response(),
    }
}

/// Read logs from log files in chronological order with pagination.
/// Only reads files that are needed based on offset and limit.
async fn read_task_logs(
    task_id: &str,
    offset: usize,
    limit: usize,
) -> Result<Vec<String>, std::io::Error> {
    let log_dir = babata_dir()
        .map_err(|e| std::io::Error::other(e.to_string()))?
        .join("logs");

    // Collect all log files with their metadata
    let mut log_files: Vec<(std::path::PathBuf, std::time::SystemTime)> = Vec::new();

    let mut entries = tokio::fs::read_dir(&log_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().map(|e| e == "log").unwrap_or(false) && path.is_file() {
            // Get file modification time for sorting
            let metadata = tokio::fs::metadata(&path).await?;
            let modified = metadata
                .modified()
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            log_files.push((path, modified));
        }
    }

    // Sort by modification time: oldest first (chronological order)
    log_files.sort_by(|a, b| a.1.cmp(&b.1));

    let target_start = offset;
    let target_end = offset.saturating_add(limit);
    let mut current_line_count: usize = 0;
    let mut result: Vec<String> = Vec::new();

    // Iterate through files in chronological order
    for (path, _) in log_files {
        // Check if we've collected enough lines
        if current_line_count >= target_end {
            break;
        }

        // Read file line by line to count matching lines
        match tokio::fs::read_to_string(&path).await {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                let mut matching_lines_in_file: Vec<String> = Vec::new();

                // Filter lines containing task_id
                for line in &lines {
                    if line.contains(task_id) {
                        matching_lines_in_file.push(line.to_string());
                    }
                }

                let file_matching_count = matching_lines_in_file.len();

                // Check if this file contains lines we need
                let file_start = current_line_count;
                let file_end = current_line_count.saturating_add(file_matching_count);

                // If there's overlap between [file_start, file_end) and [target_start, target_end)
                if file_start < target_end && file_end > target_start {
                    // Calculate which lines from this file to take
                    let skip_in_file = target_start.saturating_sub(file_start);
                    let take_in_file = std::cmp::min(
                        file_matching_count.saturating_sub(skip_in_file),
                        target_end.saturating_sub(std::cmp::max(file_start, target_start)),
                    );

                    // Add the relevant lines to result
                    result.extend(
                        matching_lines_in_file
                            .into_iter()
                            .skip(skip_in_file)
                            .take(take_in_file),
                    );
                }

                current_line_count += file_matching_count;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                continue;
            }
            Err(e) => {
                log::warn!("Failed to read log file {:?}: {}", path, e);
                continue;
            }
        }
    }

    Ok(result)
}
