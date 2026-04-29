use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;

use crate::{BabataResult, error::BabataError, utils::babata_dir};

use super::{HttpApp, ensure_task_exists, parse_task_id};

const MAX_LIMIT: usize = 1000;

/// Supported log levels for filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

impl LogLevel {
    fn matches(&self, log: &str) -> bool {
        let upper_log = log.to_ascii_uppercase();
        // Log lines may be in logforth default format:
        //   "2024-01-01T12:00:00   INFO module:file:line [task-id] message"
        // or bracketed format (tests/back-compat):
        //   "2024-01-01 [task-id] [INFO] message"
        let second_token = upper_log.split_whitespace().nth(1);
        match self {
            LogLevel::Error => upper_log.contains("[ERROR]") || second_token == Some("ERROR"),
            LogLevel::Warn => {
                upper_log.contains("[WARN]")
                    || upper_log.contains("[WARNING]")
                    || second_token == Some("WARN")
            }
            LogLevel::Info => upper_log.contains("[INFO]") || second_token == Some("INFO"),
            LogLevel::Debug => upper_log.contains("[DEBUG]") || second_token == Some("DEBUG"),
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct LogQueryParams {
    /// Required: Maximum number of log lines to return (1-1000)
    limit: usize,
    /// Optional: Number of lines to skip (default: 0)
    #[serde(default)]
    offset: usize,
    /// Optional: Filter by log level (ERROR, WARN, INFO, DEBUG)
    level: Option<LogLevel>,
}

pub(super) async fn handle(
    State(state): State<HttpApp>,
    Path(task_id): Path<String>,
    Query(params): Query<LogQueryParams>,
) -> BabataResult<Json<Vec<String>>> {
    let task_id = parse_task_id(&task_id)?;
    ensure_task_exists(&state.task_manager, task_id)?;

    if params.limit == 0 {
        return Err(BabataError::invalid_input("limit must be greater than 0"));
    }
    if params.limit > MAX_LIMIT {
        return Err(BabataError::invalid_input(format!(
            "limit exceeds maximum value of {}",
            MAX_LIMIT
        )));
    }

    let level_filter = params.level;

    let logs = read_task_logs(
        &task_id.to_string(),
        params.offset,
        params.limit,
        level_filter,
    )
    .await
    .map_err(|err| BabataError::invalid_input(format!("Failed to read logs: {}", err)))?;
    Ok(Json(logs))
}

/// Read logs from log files in chronological order with pagination.
/// Only reads files that are needed based on offset and limit.
async fn read_task_logs(
    task_id: &str,
    offset: usize,
    limit: usize,
    level_filter: Option<LogLevel>,
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
    log_files.sort_by_key(|entry| entry.1);

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

                // Filter lines containing [task_id]
                let task_marker = format!("[{}]", task_id);
                for line in &lines {
                    if line.contains(&task_marker) {
                        // Apply level filter if specified
                        if let Some(ref level) = level_filter
                            && !level.matches(line)
                        {
                            continue;
                        }
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

#[cfg(test)]
mod tests {
    use super::LogLevel;

    #[test]
    fn log_level_deserialization_standard_case() {
        assert_eq!(
            serde_json::from_str::<LogLevel>("\"ERROR\"").unwrap(),
            LogLevel::Error
        );
        assert_eq!(
            serde_json::from_str::<LogLevel>("\"WARN\"").unwrap(),
            LogLevel::Warn
        );
        assert_eq!(
            serde_json::from_str::<LogLevel>("\"INFO\"").unwrap(),
            LogLevel::Info
        );
        assert_eq!(
            serde_json::from_str::<LogLevel>("\"DEBUG\"").unwrap(),
            LogLevel::Debug
        );
    }

    #[test]
    fn log_level_deserialization_rejects_invalid_or_lowercase() {
        assert!(serde_json::from_str::<LogLevel>("\"error\"").is_err());
        assert!(serde_json::from_str::<LogLevel>("\"WARNING\"").is_err());
        assert!(serde_json::from_str::<LogLevel>("\"TRACE\"").is_err());
    }

    #[test]
    fn log_level_matches_correctly() {
        assert!(LogLevel::Error.matches("2024-01-01 [task-id] [ERROR] something failed"));
        assert!(!LogLevel::Error.matches("2024-01-01 [task-id] [INFO] ERROR in request body"));
        assert!(!LogLevel::Error.matches("2024-01-01 [task-id] [WARN] error sending request"));
        assert!(!LogLevel::Error.matches("2024-01-01 [task-id] [INFO] all good"));

        assert!(LogLevel::Warn.matches("2024-01-01 [task-id] [WARN] caution"));
        assert!(LogLevel::Warn.matches("2024-01-01 [task-id] [WARNING] caution"));
        assert!(!LogLevel::Warn.matches("2024-01-01 [task-id] [ERROR] failed"));

        assert!(LogLevel::Info.matches("2024-01-01 [task-id] [INFO] started"));
        assert!(!LogLevel::Info.matches("2024-01-01 [task-id] [DEBUG] trace"));

        assert!(LogLevel::Debug.matches("2024-01-01 [task-id] [DEBUG] trace"));
        assert!(!LogLevel::Debug.matches("2024-01-01 [task-id] [INFO] normal"));
    }

    #[test]
    fn log_level_matches_logforth_default_format() {
        // logforth TextLayout default: "2026-04-28T00:00:07.638041+08:00   INFO module:file:line [task-id] message"
        let info_line = "2026-04-28T00:00:07.638041+08:00   INFO babata::agent::runner: runner.rs:96 [task-id] Provider returned message";
        let warn_line = "2026-04-28T00:00:07.638041+08:00   WARN babata::http: get_task_logs.rs:42 [task-id] caution";
        let error_line = "2026-04-28T00:00:07.638041+08:00   ERROR babata::task: manager.rs:15 [task-id] something failed";
        let debug_line =
            "2026-04-28T00:00:07.638041+08:00   DEBUG babata::tool: shell.rs:8 [task-id] trace";

        assert!(LogLevel::Info.matches(info_line));
        assert!(!LogLevel::Error.matches(info_line));
        assert!(!LogLevel::Warn.matches(info_line));
        assert!(!LogLevel::Debug.matches(info_line));

        assert!(LogLevel::Warn.matches(warn_line));
        assert!(!LogLevel::Error.matches(warn_line));
        assert!(!LogLevel::Info.matches(warn_line));

        assert!(LogLevel::Error.matches(error_line));
        assert!(!LogLevel::Info.matches(error_line));

        assert!(LogLevel::Debug.matches(debug_line));
        assert!(!LogLevel::Info.matches(debug_line));
    }

    #[test]
    fn log_level_does_not_match_level_in_message_body_for_logforth_format() {
        // The word "ERROR" appears in the message body, but the actual level is INFO.
        let line = "2026-04-28T00:00:07.638041+08:00   INFO babata::agent::runner: runner.rs:96 [task-id] ERROR in request body";
        assert!(!LogLevel::Error.matches(line));
        assert!(LogLevel::Info.matches(line));
    }
}
