use std::time::Duration;

use serde_json::{Value, json};
use tokio::time::sleep;
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
                    "Block until a task reaches the target status by querying the local TaskStore. Use this when the next step depends on that task finishing or changing state."
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "task_id": {
                            "type": "string",
                            "description": "The UUID of the task to watch"
                        },
                        "task_status": {
                            "type": "string",
                            "description": "The target task status to wait for: running, done, canceled, or paused"
                        }
                    },
                    "required": ["task_id", "task_status"]
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

    async fn execute(&self, args: &str, _context: &ToolContext) -> BabataResult<String> {
        let args: Value = serde_json::from_str(args)?;
        let task_id = args["task_id"]
            .as_str()
            .ok_or_else(|| BabataError::tool("Missing required parameter: task_id"))?;
        let target_status = args["task_status"]
            .as_str()
            .ok_or_else(|| BabataError::tool("Missing required parameter: task_status"))?;

        let task_id = Uuid::parse_str(task_id)
            .map_err(|err| BabataError::tool(format!("Invalid task_id '{}': {}", task_id, err)))?;
        let target_status = target_status.parse::<TaskStatus>().map_err(|err| {
            BabataError::tool(format!("Invalid task_status '{}': {}", target_status, err))
        })?;

        loop {
            let task = self.task_store.get_task(task_id).map_err(|err| {
                BabataError::tool(format!("Failed to query task '{}': {}", task_id, err))
            })?;
            let current_status = task.status;

            if current_status == target_status {
                return serde_json::to_string(&task).map_err(|err| {
                    BabataError::tool(format!("Failed to serialize task '{}': {}", task_id, err))
                });
            }

            if is_unreachable_terminal_status(current_status, target_status) {
                return Err(BabataError::tool(format!(
                    "Task '{}' reached terminal status '{}' before target status '{}'",
                    task_id, current_status, target_status
                )));
            }

            sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
        }
    }
}

fn is_unreachable_terminal_status(current_status: TaskStatus, target_status: TaskStatus) -> bool {
    matches!(current_status, TaskStatus::Done | TaskStatus::Canceled)
        && current_status != target_status
}
