use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    BabataResult,
    agent::babata::{Tool, ToolContext, ToolSpec},
    error::BabataError,
    task::TaskStore,
};

#[derive(Debug)]
pub struct UpdateTaskDescriptionTool {
    spec: ToolSpec,
    task_store: TaskStore,
}

impl UpdateTaskDescriptionTool {
    pub fn new() -> BabataResult<Self> {
        Ok(Self {
            spec: ToolSpec {
                name: "update_task_description".to_string(),
                description:
                    "Update a task description in the local TaskStore. If task_id is omitted, update the current task. Use this to keep task summaries accurate as work evolves."
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "task_id": {
                            "type": "string",
                            "description": "Optional UUID of the task to update. If omitted, the current task is used."
                        },
                        "description": {
                            "type": "string",
                            "description": "The new task description"
                        }
                    },
                    "required": ["description"]
                }),
            },
            task_store: TaskStore::new()?,
        })
    }
}

#[async_trait::async_trait]
impl Tool for UpdateTaskDescriptionTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, context: &ToolContext<'_>) -> BabataResult<String> {
        let args: Value = serde_json::from_str(args)?;
        let task_id = match args["task_id"].as_str() {
            Some(task_id) => Uuid::parse_str(task_id).map_err(|err| {
                BabataError::tool(format!("Invalid task_id '{}': {}", task_id, err))
            })?,
            None => *context.task_id,
        };
        let description = args["description"]
            .as_str()
            .ok_or_else(|| BabataError::tool("Missing required parameter: description"))?
            .trim()
            .to_string();

        if description.is_empty() {
            return Err(BabataError::tool("description cannot be empty"));
        }

        self.task_store
            .update_task_description(task_id, description.clone())?;

        Ok(format!(
            "Updated description for task '{}': {}",
            task_id, description
        ))
    }
}
