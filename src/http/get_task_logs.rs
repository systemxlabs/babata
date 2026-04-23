use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;

use crate::{BabataResult, error::BabataError, utils::babata_dir};

use super::{HttpApp, ensure_task_exists, parse_task_id};

const MAX_LIMIT: usize = 1000;

/// Supported log levels for filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

#[allow(dead_code)]
#[derive(serde::Deserialize)]
#[serde(rename_all = "UPPERCASE")]
enum LogLevelRemote {
    Error,
    Warn,
    Info,
    Debug,
}

impl<'de> serde::Deserialize<'de> for LogLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<LogLevel>().map_err(serde::de::Error::custom)
    }
}

impl std::str::FromStr for LogLevel {
    type Err = BabataError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "ERROR" => Ok(LogLevel::Error),
            "WARN" | "WARNING" => Ok(LogLevel::Warn),
            "INFO" => Ok(LogLevel::Info),
            "DEBUG" => Ok(LogLevel::Debug),
            _ => Err(BabataError::invalid_input(format!(
                "Invalid log level '{}'. Supported: ERROR, WARN, INFO, DEBUG",
                s
            ))),
        }
    }
}

impl LogLevel {
    fn matches(&self, log: &str) -> bool {
        let upper_log = log.to_ascii_uppercase();
        match self {
            LogLevel::Error => upper_log.contains("[ERROR]"),
            LogLevel::Warn => upper_log.contains("[WARN]") || upper_log.contains("[WARNING]"),
            LogLevel::Info => upper_log.contains("[INFO]"),
            LogLevel::Debug => upper_log.contains("[DEBUG]"),
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct LogQueryParams {
    limit: usize,
    #[serde(default)]
    offset: usize,
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

async fn read_task_logs(
    task_id: &str,
    offset: usize,
    limit: usize,
    level_filter: Option<LogLevel>,
) -> Result<Vec<String>, std::io::Error> {
    let log_dir = babata_dir()
        .map_err(|e| std::io::Error::other(e.to_string()))?
        .join("logs");

    let mut log_files: Vec<(std::path::PathBuf, std::time::SystemTime)> = Vec::new();

    let mut entries = tokio::fs::read_dir(&log_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().map(|e| e == "log").unwrap_or(false) && path.is_file() {
            let metadata = tokio::fs::metadata(&path).await?;
            let modified = metadata
                .modified()
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            log_files.push((path, modified));
        }
    }

    log_files.sort_by_key(|entry| entry.1);

    let target_start = offset;
    let target_end = offset.saturating_add(limit);
    let mut current_line_count: usize = 0;
    let mut result: Vec<String> = Vec::new();

    for (path, _) in log_files {
        if current_line_count >= target_end {
            break;
        }

        match tokio::fs::read_to_string(&path).await {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                let mut matching_lines_in_file: Vec<String> = Vec::new();

                let task_marker = format!("[{}]", task_id);
                for line in &lines {
                    if line.contains(&task_marker) {
                        if let Some(ref level) = level_filter
                            && !level.matches(line)
                        {
                            continue;
                        }
                        matching_lines_in_file.push(line.to_string());
                    }
                }

                let file_matching_count = matching_lines_in_file.len();
                let file_start = current_line_count;
                let file_end = current_line_count.saturating_add(file_matching_count);

                if file_start < target_end && file_end > target_start {
                    let skip_in_file = target_start.saturating_sub(file_start);
                    let take_in_file = std::cmp::min(
                        file_matching_count.saturating_sub(skip_in_file),
                        target_end.saturating_sub(std::cmp::max(file_start, target_start)),
                    );

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
    fn log_level_from_str_parses_case_insensitive() {
        assert_eq!("ERROR".parse::<LogLevel>().unwrap(), LogLevel::Error);
        assert_eq!("error".parse::<LogLevel>().unwrap(), LogLevel::Error);
        assert_eq!("WARN".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("warn".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("WARNING".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("INFO".parse::<LogLevel>().unwrap(), LogLevel::Info);
        assert_eq!("info".parse::<LogLevel>().unwrap(), LogLevel::Info);
        assert_eq!("DEBUG".parse::<LogLevel>().unwrap(), LogLevel::Debug);
        assert_eq!("debug".parse::<LogLevel>().unwrap(), LogLevel::Debug);
    }

    #[test]
    fn log_level_from_str_rejects_invalid() {
        let err = "TRACE".parse::<LogLevel>().expect_err("TRACE should fail");
        assert!(err.to_string().contains("Invalid log level"));
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
    fn test_log_query_params_deserialization() {
        // We use serde_json as a proxy to test the Deserialize implementation
        // since serde_urlencoded isn't a direct dependency.
        // In a real Axum app, serde_urlencoded is used for Query extraction.

        let json_valid = r#"{"limit": 10, "level": "ERROR"}"#;
        let res1: Result<super::LogQueryParams, _> = serde_json::from_str(json_valid);
        assert!(res1.is_ok());
        assert_eq!(res1.unwrap().level, Some(LogLevel::Error));

        let json_invalid = r#"{"limit": 10, "level": "TRACE"}"#;
        let res2: Result<super::LogQueryParams, _> = serde_json::from_str(json_invalid);
        assert!(res2.is_err());
        assert!(res2.unwrap_err().to_string().contains("Invalid log level"));

        let json_none = r#"{"limit": 10}"#;
        let res3: Result<super::LogQueryParams, _> = serde_json::from_str(json_none);
        assert!(res3.is_ok());
        assert_eq!(res3.unwrap().level, None);
    }
}
