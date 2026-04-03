use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    BabataResult,
    agent::babata::{Tool, ToolContext, ToolSpec},
    error::BabataError,
    task::TaskStore,
};

#[derive(Debug)]
pub struct ListSubtasksTool {
    spec: ToolSpec,
    task_store: TaskStore,
}

impl ListSubtasksTool {
    pub fn new() -> BabataResult<Self> {
        Ok(Self {
            spec: ToolSpec {
                name: "list_subtasks".to_string(),
                description: "List direct subtasks of a task. If parent_task_id is not provided, lists subtasks of the current task.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "parent_task_id": {
                            "type": "string",
                            "description": "Optional UUID of the parent task to list subtasks for"
                        }
                    }
                }),
            },
            task_store: TaskStore::new()?,
        })
    }
}

#[async_trait::async_trait]
impl Tool for ListSubtasksTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, context: &ToolContext<'_>) -> BabataResult<String> {
        let parent_task_id = parse_args(args, context)?;

        let tasks = self.task_store.list_subtasks(parent_task_id)?;
        serde_json::to_string(&tasks).map_err(Into::into)
    }
}

fn parse_args(args: &str, context: &ToolContext<'_>) -> BabataResult<Uuid> {
    let args: Value = serde_json::from_str(args)?;

    let parent_task_id = match args.get("parent_task_id") {
        Some(value) => {
            let task_id_str = value
                .as_str()
                .ok_or_else(|| BabataError::tool("Parameter 'parent_task_id' must be a string"))?;
            Uuid::parse_str(task_id_str).map_err(|err| {
                BabataError::tool(format!("Invalid parent_task_id '{}': {}", task_id_str, err))
            })?
        }
        None => *context.task_id,
    };

    Ok(parent_task_id)
}
