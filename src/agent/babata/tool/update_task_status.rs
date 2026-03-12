use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    BabataResult,
    agent::babata::{Tool, ToolSpec},
    error::BabataError,
    task::{TaskStatus, TaskStore},
};

#[derive(Debug)]
pub struct UpdateTaskStatusTool {
    spec: ToolSpec,
    store: TaskStore,
}

impl UpdateTaskStatusTool {
    pub fn new() -> BabataResult<Self> {
        Ok(Self {
            spec: ToolSpec {
                name: "update_task_status".to_string(),
                description:
                    "Update a task status in the task store. Valid statuses: running, done, canceled, paused."
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "task_id": {
                            "type": "string",
                            "description": "The UUID of the task to update"
                        },
                        "status": {
                            "type": "string",
                            "description": "New task status: running, done, canceled, or paused"
                        }
                    },
                    "required": ["task_id", "status"]
                }),
            },
            store: TaskStore::new()?,
        })
    }
}

#[async_trait::async_trait]
impl Tool for UpdateTaskStatusTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str) -> BabataResult<String> {
        let args: Value = serde_json::from_str(args)?;
        let task_id = args["task_id"]
            .as_str()
            .ok_or_else(|| BabataError::tool("Missing required parameter: task_id"))?;
        let status = args["status"]
            .as_str()
            .ok_or_else(|| BabataError::tool("Missing required parameter: status"))?;

        let task_id = Uuid::parse_str(task_id)
            .map_err(|err| BabataError::tool(format!("Invalid task_id '{}': {}", task_id, err)))?;
        let status = status.parse::<TaskStatus>().map_err(|err| {
            BabataError::tool(format!("Invalid task status '{}': {}", status, err))
        })?;

        self.store.update_task_status(task_id, status)?;

        Ok(format!(
            "Updated task '{}' status to '{}'",
            task_id,
            status.as_str()
        ))
    }
}
