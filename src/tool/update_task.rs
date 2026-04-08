use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    BabataResult,
    error::BabataError,
    task::{TaskStatus, TaskStore},
    tool::{Tool, ToolContext, ToolSpec, parse_tool_args},
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
                parameters: schemars::schema_for!(UpdateTaskArgs),
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
        let (task_id, description, never_ends) = validate_args(args, context)?;

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

fn validate_args(
    args: &str,
    context: &ToolContext<'_>,
) -> BabataResult<(Uuid, Option<String>, Option<bool>)> {
    let args: UpdateTaskArgs = parse_tool_args(args)?;
    let task_id = args.task_id.unwrap_or(*context.task_id);
    let description = args.description.map(|value| value.trim().to_string());
    let never_ends = args.never_ends;

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

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct UpdateTaskArgs {
    #[schemars(
        description = "Optional UUID of the task to update. If omitted, the current task is used."
    )]
    task_id: Option<Uuid>,
    #[schemars(description = "Optional new task description")]
    description: Option<String>,
    #[schemars(description = "Optional boolean flag to update on the task")]
    never_ends: Option<bool>,
}
