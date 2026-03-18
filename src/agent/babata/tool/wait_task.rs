use std::time::Duration;

use serde_json::{Value, json};
use tokio::time::{Instant, sleep};
use uuid::Uuid;

use crate::{
    BabataResult,
    agent::babata::{Tool, ToolContext, ToolSpec},
    error::BabataError,
    task::{TaskStatus, TaskStore},
};

const POLL_INTERVAL_SECS: u64 = 30;

#[derive(Debug)]
pub struct WaitTaskTool {
    spec: ToolSpec,
    task_store: TaskStore,
}

impl WaitTaskTool {
    pub fn new() -> BabataResult<Self> {
        Ok(Self {
            spec: ToolSpec {
                name: "wait_task".to_string(),
                description:
                    "Block until a task reaches any target status by querying the local TaskStore. Supports waiting for one or more statuses and an optional timeout. Use this when the next step depends on that task finishing or changing state."
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "task_id": {
                            "type": "string",
                            "description": "The UUID of the task to watch"
                        },
                        "task_statuses": {
                            "type": "array",
                            "description": "One or more target task statuses to wait for: running, done, canceled, or paused",
                            "items": {
                                "type": "string"
                            },
                            "minItems": 1
                        },
                        "timeout_secs": {
                            "type": "integer",
                            "description": "Optional timeout in seconds for the wait operation",
                            "minimum": 0
                        }
                    },
                    "required": ["task_id", "task_statuses"]
                }),
            },
            task_store: TaskStore::new()?,
        })
    }
}

#[async_trait::async_trait]
impl Tool for WaitTaskTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, _context: &ToolContext<'_>) -> BabataResult<String> {
        let args: Value = serde_json::from_str(args)?;
        let task_id = parse_task_id(&args)?;
        let target_statuses = parse_target_statuses(&args)?;
        let timeout = parse_timeout(&args)?;
        let deadline = timeout.map(|timeout| Instant::now() + timeout);

        loop {
            let task = self.task_store.get_task(task_id).map_err(|err| {
                BabataError::tool(format!("Failed to query task '{}': {}", task_id, err))
            })?;
            let current_status = task.status;

            if target_statuses.contains(&current_status) {
                return serde_json::to_string(&task).map_err(|err| {
                    BabataError::tool(format!("Failed to serialize task '{}': {}", task_id, err))
                });
            }

            if is_unreachable_terminal_status(current_status, &target_statuses) {
                return Err(BabataError::tool(format!(
                    "Task '{}' reached terminal status '{}' before target statuses [{}]",
                    task_id,
                    current_status,
                    format_target_statuses(&target_statuses)
                )));
            }

            let sleep_duration = match remaining_sleep(deadline) {
                Some(duration) => duration,
                None => {
                    return Err(BabataError::tool(format!(
                        "Timed out waiting for task '{}' to reach statuses [{}]",
                        task_id,
                        format_target_statuses(&target_statuses)
                    )));
                }
            };
            sleep(sleep_duration).await;
        }
    }
}

fn parse_task_id(args: &Value) -> BabataResult<Uuid> {
    let task_id = args["task_id"]
        .as_str()
        .ok_or_else(|| BabataError::tool("Missing required parameter: task_id"))?;
    Uuid::parse_str(task_id)
        .map_err(|err| BabataError::tool(format!("Invalid task_id '{}': {}", task_id, err)))
}

fn parse_target_statuses(args: &Value) -> BabataResult<Vec<TaskStatus>> {
    let task_statuses = args
        .get("task_statuses")
        .ok_or_else(|| BabataError::tool("Missing required parameter: task_statuses"))?;
    let statuses = task_statuses
        .as_array()
        .ok_or_else(|| {
            BabataError::tool("Parameter 'task_statuses' must be a non-empty array of strings")
        })?
        .iter()
        .map(|value| {
            let status = value.as_str().ok_or_else(|| {
                BabataError::tool("Parameter 'task_statuses' must be a non-empty array of strings")
            })?;
            parse_task_status(status, "task_statuses")
        })
        .collect::<BabataResult<Vec<_>>>()?;

    if statuses.is_empty() {
        return Err(BabataError::tool(
            "Parameter 'task_statuses' must contain at least one status",
        ));
    }

    Ok(statuses)
}

fn parse_task_status(value: &str, parameter_name: &str) -> BabataResult<TaskStatus> {
    value.parse::<TaskStatus>().map_err(|err| {
        BabataError::tool(format!("Invalid {} '{}': {}", parameter_name, value, err))
    })
}

fn parse_timeout(args: &Value) -> BabataResult<Option<Duration>> {
    let Some(timeout_secs) = args.get("timeout_secs") else {
        return Ok(None);
    };

    let timeout_secs = timeout_secs.as_u64().ok_or_else(|| {
        BabataError::tool("Parameter 'timeout_secs' must be a non-negative integer")
    })?;
    Ok(Some(Duration::from_secs(timeout_secs)))
}

fn is_unreachable_terminal_status(
    current_status: TaskStatus,
    target_statuses: &[TaskStatus],
) -> bool {
    matches!(current_status, TaskStatus::Done | TaskStatus::Canceled)
        && !target_statuses.contains(&current_status)
}

fn remaining_sleep(deadline: Option<Instant>) -> Option<Duration> {
    match deadline {
        None => Some(Duration::from_secs(POLL_INTERVAL_SECS)),
        Some(deadline) => {
            let now = Instant::now();
            if now >= deadline {
                return None;
            }

            Some(std::cmp::min(
                Duration::from_secs(POLL_INTERVAL_SECS),
                deadline - now,
            ))
        }
    }
}

fn format_target_statuses(target_statuses: &[TaskStatus]) -> String {
    target_statuses
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::{
        POLL_INTERVAL_SECS, format_target_statuses, is_unreachable_terminal_status,
        parse_target_statuses, parse_timeout, remaining_sleep,
    };
    use crate::task::TaskStatus;
    use serde_json::json;
    use std::time::Duration;
    use tokio::time::Instant;

    #[test]
    fn parse_target_statuses_accepts_plural_parameter() {
        let args = json!({
            "task_statuses": ["done", "canceled"]
        });

        let statuses = parse_target_statuses(&args).expect("task statuses");
        assert_eq!(statuses, vec![TaskStatus::Done, TaskStatus::Canceled]);
    }

    #[test]
    fn parse_target_statuses_rejects_missing_status_parameters() {
        let args = json!({});

        let err = parse_target_statuses(&args).expect_err("missing status should fail");
        assert!(err.to_string().contains("task_statuses"));
    }

    #[test]
    fn parse_timeout_accepts_non_negative_integer() {
        let args = json!({
            "timeout_secs": 15
        });

        let timeout = parse_timeout(&args).expect("timeout");
        assert_eq!(timeout, Some(Duration::from_secs(15)));
    }

    #[test]
    fn parse_timeout_rejects_invalid_type() {
        let args = json!({
            "timeout_secs": "15"
        });

        let err = parse_timeout(&args).expect_err("invalid timeout should fail");
        assert!(err.to_string().contains("timeout_secs"));
    }

    #[test]
    fn unreachable_terminal_status_respects_target_statuses() {
        assert!(is_unreachable_terminal_status(
            TaskStatus::Done,
            &[TaskStatus::Canceled]
        ));
        assert!(!is_unreachable_terminal_status(
            TaskStatus::Done,
            &[TaskStatus::Done, TaskStatus::Canceled]
        ));
        assert!(!is_unreachable_terminal_status(
            TaskStatus::Paused,
            &[TaskStatus::Done]
        ));
    }

    #[test]
    fn remaining_sleep_caps_by_poll_interval() {
        let deadline = Instant::now() + Duration::from_secs(POLL_INTERVAL_SECS * 2);
        let duration = remaining_sleep(Some(deadline)).expect("sleep duration");
        assert_eq!(duration, Duration::from_secs(POLL_INTERVAL_SECS));
    }

    #[test]
    fn remaining_sleep_uses_remaining_timeout() {
        let deadline = Instant::now() + Duration::from_secs(5);
        let duration = remaining_sleep(Some(deadline)).expect("sleep duration");
        assert!(duration <= Duration::from_secs(5));
    }

    #[test]
    fn remaining_sleep_returns_none_after_deadline() {
        let deadline = Instant::now();
        assert!(remaining_sleep(Some(deadline)).is_none());
    }

    #[test]
    fn format_target_statuses_joins_statuses() {
        let statuses = vec![TaskStatus::Done, TaskStatus::Canceled];
        assert_eq!(format_target_statuses(&statuses), "done, canceled");
    }
}
