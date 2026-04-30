use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};

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
    fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
        }
    }

    fn matches_precise(&self, record_level: &str) -> bool {
        self.as_str() == record_level
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

/// A single log entry returned by the task logs API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub file: String,
    pub line: u32,
    pub message: String,
}

/// Response body for the task logs endpoint (replaces `Vec<String>`).
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct GetTaskLogsResponse {
    pub logs: Vec<LogEntry>,
}

pub(super) async fn handle(
    State(state): State<HttpApp>,
    Path(task_id): Path<String>,
    Query(params): Query<LogQueryParams>,
) -> BabataResult<Json<GetTaskLogsResponse>> {
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
    Ok(Json(GetTaskLogsResponse { logs }))
}

/// Process a single log line through the "coarse filter → deserialize → precise filter" pipeline.
///
/// Returns `Some(LogEntry)` when the line belongs to the requested task and passes the level filter.
/// Non-JSON lines or lines that fail any filter stage are silently dropped.
fn process_log_line(
    line: &str,
    task_id: &str,
    level_filter: Option<&LogLevel>,
) -> Option<LogEntry> {
    let task_marker = format!("[{}]", task_id);

    // Coarse filter 1: check if line contains the task marker
    if !line.contains(&task_marker) {
        return None;
    }

    // Coarse filter 2: if level filter is present, do a quick
    // string check to avoid parsing lines that definitely don't match
    if let Some(level) = level_filter
        && !line.contains(level.as_str())
    {
        return None;
    }

    // Deserialize the JSON log line directly into LogEntry
    let entry: LogEntry = serde_json::from_str(line).ok()?;

    // Precise filter 1: confirm message contains task_id
    if !entry.message.contains(&task_marker) {
        return None;
    }

    // Precise filter 2: if level filter is present,
    // perform exact level comparison
    if let Some(level) = level_filter
        && !level.matches_precise(&entry.level)
    {
        return None;
    }

    Some(entry)
}

/// Read logs from log files in chronological order with pagination.
/// Only reads files that are needed based on offset and limit.
async fn read_task_logs(
    task_id: &str,
    offset: usize,
    limit: usize,
    level_filter: Option<LogLevel>,
) -> Result<Vec<LogEntry>, std::io::Error> {
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
    let mut result: Vec<LogEntry> = Vec::new();

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
                let mut matching_lines_in_file: Vec<LogEntry> = Vec::new();

                for line in &lines {
                    if let Some(entry) = process_log_line(line, task_id, level_filter.as_ref()) {
                        matching_lines_in_file.push(entry);
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
    use super::{LogEntry, LogLevel};

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
    fn precise_level_filtering_matches_correctly() {
        assert!(LogLevel::Error.matches_precise("ERROR"));
        assert!(!LogLevel::Error.matches_precise("INFO"));
        assert!(LogLevel::Warn.matches_precise("WARN"));
        assert!(!LogLevel::Warn.matches_precise("WARNING"));
        assert!(LogLevel::Info.matches_precise("INFO"));
        assert!(LogLevel::Debug.matches_precise("DEBUG"));
    }

    #[test]
    fn message_keyword_false_positive_is_avoided() {
        // A log line where the message body contains "ERROR" but the real level is INFO.
        // With the old substring matching this would be a false positive for LogLevel::Error.
        let line = r#"{"timestamp":"2026-04-29T10:00:00+08:00","level":"INFO","target":"t","file":"f.rs","line":1,"message":"[task-1] ERROR in request body"}"#;
        let entry: LogEntry = serde_json::from_str(line).unwrap();
        assert!(!LogLevel::Error.matches_precise(&entry.level));
        assert_eq!(entry.level, "INFO");
    }

    #[test]
    fn non_json_line_is_dropped() {
        let legacy = "2024-01-01 [task-id] [INFO] old format log line";
        let result = super::process_log_line(legacy, "task-id", None);
        assert!(
            result.is_none(),
            "non-JSON lines must be dropped without legacy fallback"
        );
    }

    #[test]
    fn log_entry_serialization_roundtrip() {
        let entry = LogEntry {
            timestamp: "2026-04-29T10:00:00+08:00".to_string(),
            level: "INFO".to_string(),
            target: "babata::test".to_string(),
            file: "test.rs".to_string(),
            line: 42,
            message: "[task-1] hello".to_string(),
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        assert!(json.contains("\"level\":\"INFO\""));
        assert!(json.contains("\"message\":\"[task-1] hello\""));
    }

    #[test]
    fn log_level_as_str_returns_uppercase() {
        assert_eq!(LogLevel::Error.as_str(), "ERROR");
        assert_eq!(LogLevel::Warn.as_str(), "WARN");
        assert_eq!(LogLevel::Info.as_str(), "INFO");
        assert_eq!(LogLevel::Debug.as_str(), "DEBUG");
    }

    #[test]
    fn process_log_line_json_parsing_and_filtering() {
        let info_line = r#"{"timestamp":"2026-04-29T10:00:00+08:00","level":"INFO","target":"t","file":"f.rs","line":1,"message":"[abc] started"}"#;
        let error_line = r#"{"timestamp":"2026-04-29T10:00:01+08:00","level":"ERROR","target":"t","file":"f.rs","line":2,"message":"[abc] failed"}"#;
        let other_task = r#"{"timestamp":"2026-04-29T10:00:02+08:00","level":"INFO","target":"t","file":"f.rs","line":3,"message":"[xyz] other task"}"#;

        // No level filter
        let e1 = super::process_log_line(info_line, "abc", None);
        assert!(e1.is_some());
        assert_eq!(e1.unwrap().level, "INFO");

        let e2 = super::process_log_line(error_line, "abc", None);
        assert!(e2.is_some());
        assert_eq!(e2.unwrap().level, "ERROR");

        let e3 = super::process_log_line(other_task, "abc", None);
        assert!(e3.is_none());

        // With level filter = Error
        let e4 = super::process_log_line(info_line, "abc", Some(&LogLevel::Error));
        assert!(
            e4.is_none(),
            "INFO line should be filtered out by Error level"
        );

        let e5 = super::process_log_line(error_line, "abc", Some(&LogLevel::Error));
        assert!(e5.is_some());
        assert_eq!(e5.unwrap().level, "ERROR");
    }

    #[test]
    fn process_log_line_message_keyword_false_positive() {
        // Message contains "ERROR" but actual level is WARN.
        let line = r#"{"timestamp":"2026-04-29T10:00:00+08:00","level":"WARN","target":"t","file":"f.rs","line":1,"message":"[abc] Failed to parse ERROR response"}"#;

        let result = super::process_log_line(line, "abc", Some(&LogLevel::Error));
        assert!(
            result.is_none(),
            "should not match Error level when actual level is WARN"
        );

        let result_warn = super::process_log_line(line, "abc", Some(&LogLevel::Warn));
        assert!(result_warn.is_some());
        assert_eq!(result_warn.unwrap().level, "WARN");
    }

    #[test]
    fn process_log_line_non_json_is_dropped() {
        let legacy = "2024-01-01 [abc] [INFO] old format log line";

        // Without level filter: must return None (no legacy UNKNOWN fallback)
        let result = super::process_log_line(legacy, "abc", None);
        assert!(
            result.is_none(),
            "legacy line must be dropped, not returned as UNKNOWN"
        );

        // With level filter: also None
        let result_filtered = super::process_log_line(legacy, "abc", Some(&LogLevel::Info));
        assert!(
            result_filtered.is_none(),
            "legacy line must be dropped when level filter is active"
        );
    }

    #[test]
    fn process_log_line_task_id_filtering() {
        let line = r#"{"timestamp":"2026-04-29T10:00:00+08:00","level":"INFO","target":"t","file":"f.rs","line":1,"message":"[task-a] hello"}"#;

        assert!(super::process_log_line(line, "task-a", None).is_some());
        assert!(super::process_log_line(line, "task-b", None).is_none());
    }

    #[test]
    fn process_log_line_pagination_semantics_preserved() {
        // process_log_line itself does not implement pagination;
        // we verify that the filtering pipeline does not drop or reorder lines
        // in a way that would break the caller's offset/limit arithmetic.
        let lines: Vec<&str> = vec![
            r#"{"timestamp":"2026-04-29T10:00:00+08:00","level":"INFO","target":"t","file":"f.rs","line":1,"message":"[t1] line 1"}"#,
            r#"{"timestamp":"2026-04-29T10:00:01+08:00","level":"INFO","target":"t","file":"f.rs","line":2,"message":"[t1] line 2"}"#,
            r#"{"timestamp":"2026-04-29T10:00:02+08:00","level":"INFO","target":"t","file":"f.rs","line":3,"message":"[t1] line 3"}"#,
        ];

        let filtered: Vec<_> = lines
            .iter()
            .filter_map(|line| super::process_log_line(line, "t1", None))
            .collect();

        assert_eq!(filtered.len(), 3);
        assert_eq!(filtered[0].message, "[t1] line 1");
        assert_eq!(filtered[1].message, "[t1] line 2");
        assert_eq!(filtered[2].message, "[t1] line 3");
    }
}
