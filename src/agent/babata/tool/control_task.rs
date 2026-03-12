use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    BabataResult,
    agent::babata::{Tool, ToolSpec},
    error::BabataError,
    task::{TaskStatus, TaskStore},
};

#[derive(Debug)]
pub struct ControlTaskTool {
    spec: ToolSpec,
    store: TaskStore,
}

impl ControlTaskTool {
    pub fn new() -> BabataResult<Self> {
        Ok(Self {
            spec: ToolSpec {
                name: "control_task".to_string(),
                description:
                    "Control a task through a high-level action. Supported actions: pause, resume, cancel."
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "task_id": {
                            "type": "string",
                            "description": "The UUID of the task to control"
                        },
                        "action": {
                            "type": "string",
                            "description": "The control action: pause, resume, or cancel"
                        }
                    },
                    "required": ["task_id", "action"]
                }),
            },
            store: TaskStore::new()?,
        })
    }
}

#[async_trait::async_trait]
impl Tool for ControlTaskTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str) -> BabataResult<String> {
        let args: Value = serde_json::from_str(args)?;
        let task_id = args["task_id"]
            .as_str()
            .ok_or_else(|| BabataError::tool("Missing required parameter: task_id"))?;
        let action = args["action"]
            .as_str()
            .ok_or_else(|| BabataError::tool("Missing required parameter: action"))?;

        let task_id = Uuid::parse_str(task_id)
            .map_err(|err| BabataError::tool(format!("Invalid task_id '{}': {}", task_id, err)))?;
        let status = parse_action(action)?;

        self.store.update_task_status(task_id, status)?;

        Ok(format!(
            "Applied action '{}' to task '{}'; task status is now '{}'",
            action,
            task_id,
            status.as_str()
        ))
    }
}

fn parse_action(action: &str) -> BabataResult<TaskStatus> {
    match action {
        "pause" => Ok(TaskStatus::Paused),
        "resume" => Ok(TaskStatus::Running),
        "cancel" => Ok(TaskStatus::Canceled),
        _ => Err(BabataError::tool(format!(
            "Invalid action '{}'; expected one of: pause, resume, cancel",
            action
        ))),
    }
}
