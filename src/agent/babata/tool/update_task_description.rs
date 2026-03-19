use serde_json::{Value, json};

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
                    "Update the current task description in the local TaskStore. Use this to keep the task summary accurate as the task evolves."
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
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
        let description = args["description"]
            .as_str()
            .ok_or_else(|| BabataError::tool("Missing required parameter: description"))?
            .trim()
            .to_string();

        if description.is_empty() {
            return Err(BabataError::tool("description cannot be empty"));
        }

        self.task_store
            .update_task_description(*context.task_id, description.clone())?;

        Ok(format!(
            "Updated description for task '{}': {}",
            context.task_id, description
        ))
    }
}
