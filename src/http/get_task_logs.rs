use axum::{
    Json,
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::utils::babata_dir;

use super::{ApiError, HttpApp};

#[derive(Debug, Deserialize)]
pub(crate) struct LogQueryParams {
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    offset: usize,
}

fn default_limit() -> usize {
    100
}

#[derive(Debug, Serialize)]
pub(crate) struct TaskLogsResponse {
    task_id: String,
    total_lines: usize,
    offset: usize,
    limit: usize,
    has_more: bool,
    logs: Vec<String>,
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

    // Clamp limit to prevent excessive memory usage
    let limit = params.limit.min(1000);
    let offset = params.offset;

    // Read and filter logs
    match read_task_logs(&task_id_str, offset, limit).await {
        Ok((logs, total_lines)) => {
            let has_more = offset + logs.len() < total_lines;
            Json(TaskLogsResponse {
                task_id: task_id_str,
                total_lines,
                offset,
                limit,
                has_more,
                logs,
            })
            .into_response()
        }
        Err(err) => ApiError::bad_request(format!("Failed to read logs: {}", err)).into_response(),
    }
}

/// Read logs from the log file and filter by task_id with pagination
async fn read_task_logs(
    task_id: &str,
    offset: usize,
    limit: usize,
) -> Result<(Vec<String>, usize), std::io::Error> {
    let log_dir = babata_dir()
        .map_err(|e| std::io::Error::other(e.to_string()))?
        .join("logs");

    let log_file = log_dir.join("babata.log");

    // Read the log file
    let content = match tokio::fs::read_to_string(&log_file).await {
        Ok(content) => content,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Log file doesn't exist yet
            return Ok((Vec::new(), 0));
        }
        Err(e) => return Err(e),
    };

    // Filter lines containing the task_id
    let matching_lines: Vec<String> = content
        .lines()
        .filter(|line| line.contains(task_id))
        .map(|line| line.to_string())
        .collect();

    let total_lines = matching_lines.len();

    // Apply pagination
    let paginated_logs: Vec<String> = matching_lines
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();

    Ok((paginated_logs, total_lines))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_limit() {
        assert_eq!(default_limit(), 100);
    }

    #[test]
    fn test_task_logs_response_serialization() {
        let response = TaskLogsResponse {
            task_id: "12345678-1234-1234-1234-123456789abc".to_string(),
            total_lines: 50,
            offset: 0,
            limit: 10,
            has_more: true,
            logs: vec![
                "2026-04-05T08:38:36.969579+08:00   INFO babata::task::manager: manager.rs:109 Creating task 12345678-1234-1234-1234-123456789abc".to_string(),
            ],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("task_id"));
        assert!(json.contains("total_lines"));
        assert!(json.contains("has_more"));
        assert!(json.contains("logs"));
    }
}
