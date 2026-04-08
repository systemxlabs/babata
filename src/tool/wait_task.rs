use std::time::Duration;

use schemars::JsonSchema;
use serde::Deserialize;
use tokio::time::{Instant, sleep};
use uuid::Uuid;

use crate::{
    BabataResult,
    error::BabataError,
    task::{TaskStatus, TaskStore},
    tool::{Tool, ToolContext, ToolSpec, parse_tool_args},
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
                    "Block until a task reaches any target status. Supports waiting for one or more statuses and an optional timeout. Use this when the next step depends on that task finishing or changing state."
                        .to_string(),
                parameters: schemars::schema_for!(WaitTaskArgs),
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
        let args: WaitTaskArgs = parse_tool_args(args)?;
        let target_statuses = validate_target_statuses(&args)?;
        let timeout = parse_timeout(&args);
        let deadline = timeout.map(|timeout| Instant::now() + timeout);

        loop {
            let task = self.task_store.get_task(args.task_id).map_err(|err| {
                BabataError::tool(format!("Failed to query task '{}': {}", args.task_id, err))
            })?;
            let current_status = task.status;

            if target_statuses.contains(&current_status) {
                return serde_json::to_string(&task).map_err(|err| {
                    BabataError::tool(format!(
                        "Failed to serialize task '{}': {}",
                        args.task_id, err
                    ))
                });
            }

            if is_unreachable_terminal_status(current_status, &target_statuses) {
                return Err(BabataError::tool(format!(
                    "Task '{}' reached terminal status '{}' before target statuses [{}]",
                    args.task_id,
                    current_status,
                    format_target_statuses(&target_statuses)
                )));
            }

            let sleep_duration = match remaining_sleep(deadline) {
                Some(duration) => duration,
                None => {
                    return Err(BabataError::tool(format!(
                        "Timed out waiting for task '{}' to reach statuses [{}]",
                        args.task_id,
                        format_target_statuses(&target_statuses)
                    )));
                }
            };
            sleep(sleep_duration).await;
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct WaitTaskArgs {
    #[schemars(description = "The UUID of the task to watch")]
    task_id: Uuid,
    #[schemars(
        description = "One or more target task statuses to wait for: running, done, canceled, or paused"
    )]
    task_statuses: Vec<TaskStatus>,
    #[schemars(description = "Optional timeout in seconds for the wait operation")]
    timeout_secs: Option<usize>,
}

fn validate_target_statuses(args: &WaitTaskArgs) -> BabataResult<Vec<TaskStatus>> {
    let statuses = args.task_statuses.clone();

    if statuses.is_empty() {
        return Err(BabataError::tool(
            "Parameter 'task_statuses' must contain at least one status",
        ));
    }

    Ok(statuses)
}

fn parse_timeout(args: &WaitTaskArgs) -> Option<Duration> {
    args.timeout_secs
        .map(|timeout_secs| Duration::from_secs(timeout_secs as u64))
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
        POLL_INTERVAL_SECS, WaitTaskArgs, format_target_statuses, is_unreachable_terminal_status,
        parse_timeout, remaining_sleep, validate_target_statuses,
    };
    use crate::{task::TaskStatus, tool::parse_tool_args};
    use serde_json::json;
    use std::time::Duration;
    use tokio::time::Instant;

    #[test]
    fn validate_target_statuses_accepts_plural_parameter() {
        let args = serde_json::from_value::<WaitTaskArgs>(json!({
            "task_id": uuid::Uuid::new_v4(),
            "task_statuses": ["done", "canceled"]
        }))
        .expect("wait args");

        let statuses = validate_target_statuses(&args).expect("task statuses");
        assert_eq!(statuses, vec![TaskStatus::Done, TaskStatus::Canceled]);
    }

    #[test]
    fn validate_target_statuses_rejects_missing_status_parameters() {
        let err = parse_tool_args::<WaitTaskArgs>(
            &json!({ "task_id": uuid::Uuid::new_v4() }).to_string(),
        )
        .expect_err("missing status should fail");
        assert!(err.to_string().contains("task_statuses"));
    }

    #[test]
    fn parse_timeout_accepts_non_negative_integer() {
        let args = serde_json::from_value::<WaitTaskArgs>(json!({
            "task_id": uuid::Uuid::new_v4(),
            "task_statuses": ["done"],
            "timeout_secs": 15
        }))
        .expect("wait args");

        let timeout = parse_timeout(&args);
        assert_eq!(timeout, Some(Duration::from_secs(15)));
    }

    #[test]
    fn parse_timeout_rejects_invalid_type() {
        let err = parse_tool_args::<WaitTaskArgs>(
            &json!({
                "task_id": uuid::Uuid::new_v4(),
                "task_statuses": ["done"],
                "timeout_secs": "15"
            })
            .to_string(),
        )
        .expect_err("invalid timeout should fail");
        assert!(err.to_string().contains("Invalid tool arguments"));
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
