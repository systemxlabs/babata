use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    BabataResult,
    error::BabataError,
    task::{TaskStatus, TaskStore},
    tool::{Tool, ToolContext, ToolSpec},
};

#[derive(Debug)]
pub struct UpdateTaskTool {
    spec: ToolSpec,
    task_store: TaskStore,
}

impl UpdateTaskTool {
    pub fn new() -> BabataResult<Self> {
        Ok(Self {
            spec: ToolSpec {
                name: "update_task".to_string(),
                description:
                    "Update task fields for a running or paused task. If task_id is omitted, update the current task. Use this to keep task summaries and never_ends flags accurate as work evolves."
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
                            "description": "Optional new task description"
                        },
                        "never_ends": {
                            "type": "boolean",
                            "description": "Optional boolean flag to update on the task"
                        }
                    },
                    "anyOf": [
                        { "required": ["description"] },
                        { "required": ["never_ends"] }
                    ]
                }),
            },
            task_store: TaskStore::new()?,
        })
    }
}

#[async_trait::async_trait]
impl Tool for UpdateTaskTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, context: &ToolContext<'_>) -> BabataResult<String> {
        let (task_id, description, never_ends) = parse_args(args, context)?;

        let task = self.task_store.get_task(task_id)?;
        if !matches!(task.status, TaskStatus::Running | TaskStatus::Paused) {
            return Err(BabataError::tool(format!(
                "Task '{}' cannot be updated from status '{}'; only running or paused tasks can be updated",
                task_id, task.status
            )));
        }

        self.task_store
            .update_task(task_id, description.clone(), never_ends)?;

        let mut updates = Vec::new();
        if let Some(description) = description {
            updates.push(format!("description='{}'", description));
        }
        if let Some(never_ends) = never_ends {
            updates.push(format!("never_ends={}", never_ends));
        }

        Ok(format!(
            "Updated task '{}': {}",
            task_id,
            updates.join(", ")
        ))
    }
}

fn parse_args(
    args: &str,
    context: &ToolContext<'_>,
) -> BabataResult<(Uuid, Option<String>, Option<bool>)> {
    let args: Value = serde_json::from_str(args)?;
    let task_id = match args["task_id"].as_str() {
        Some(task_id) => Uuid::parse_str(task_id)
            .map_err(|err| BabataError::tool(format!("Invalid task_id '{}': {}", task_id, err)))?,
        None => *context.task_id,
    };
    let description = match args.get("description") {
        Some(value) => Some(
            value
                .as_str()
                .ok_or_else(|| BabataError::tool("Parameter description must be a string"))?
                .trim()
                .to_string(),
        ),
        None => None,
    };
    let never_ends = match args.get("never_ends") {
        Some(value) => Some(
            value
                .as_bool()
                .ok_or_else(|| BabataError::tool("Parameter never_ends must be a boolean"))?,
        ),
        None => None,
    };

    if description.as_deref() == Some("") {
        return Err(BabataError::tool("description cannot be empty"));
    }
    if description.is_none() && never_ends.is_none() {
        return Err(BabataError::tool(
            "At least one of description or never_ends must be provided",
        ));
    }

    Ok((task_id, description, never_ends))
}
