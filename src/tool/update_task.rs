use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    BabataResult,
    error::BabataError,
    task::{TaskStatus, TaskStore, TaskUpdate},
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
                    "Update task field for a running or paused task. If task_id is omitted, update the current task. Use this to keep task summaries and never_ends flags accurate as work evolves."
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
        let args: UpdateTaskArgs = parse_tool_args(args)?;
        let task_id = args.task_id.unwrap_or(*context.task_id);

        let task = self.task_store.get_task(task_id)?;
        if !matches!(task.status, TaskStatus::Running | TaskStatus::Paused) {
            return Err(BabataError::tool(format!(
                "Task '{}' cannot be updated from status '{}'; only running or paused tasks can be updated",
                task_id, task.status
            )));
        }

        self.task_store.update_task(task_id, args.update.clone())?;

        let update_description = match &args.update {
            TaskUpdate::Description { description } => format!("description='{}'", description),
            TaskUpdate::NeverEnds { never_ends } => format!("never_ends={}", never_ends),
        };

        Ok(format!(
            "Updated task '{}': {}",
            task_id, update_description
        ))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdateTaskArgs {
    #[schemars(
        description = "Optional UUID of the task to update. If omitted, the current task is used."
    )]
    task_id: Option<Uuid>,
    #[serde(flatten)]
    update: TaskUpdate,
}

#[cfg(test)]
mod tests {
    use super::UpdateTaskArgs;
    use crate::{
        error::BabataError,
        task::TaskUpdate,
        tool::{ToolContext, parse_tool_args},
    };
    use serde_json::json;
    use uuid::Uuid;

    fn parse_update_args(
        args: &str,
        context: &ToolContext<'_>,
    ) -> Result<(Uuid, TaskUpdate), BabataError> {
        let args: UpdateTaskArgs = parse_tool_args(args)?;
        let task_id = args.task_id.unwrap_or(*context.task_id);

        match args.update {
            TaskUpdate::Description { description } => {
                let description = description.trim().to_string();
                if description.is_empty() {
                    return Err(BabataError::tool("description cannot be empty"));
                }
                Ok((task_id, TaskUpdate::Description { description }))
            }
            TaskUpdate::NeverEnds { never_ends } => {
                Ok((task_id, TaskUpdate::NeverEnds { never_ends }))
            }
        }
    }

    #[test]
    fn parse_description_update_args() {
        let args = parse_tool_args::<UpdateTaskArgs>(
            &json!({
                "field": "description",
                "description": "trim me"
            })
            .to_string(),
        )
        .expect("parse args");

        assert!(matches!(
            args,
            UpdateTaskArgs {
                update: TaskUpdate::Description { description },
                ..
            } if description == "trim me"
        ));
    }

    #[test]
    fn parse_never_ends_update_args() {
        let args = parse_tool_args::<UpdateTaskArgs>(
            &json!({
                "field": "never_ends",
                "never_ends": true
            })
            .to_string(),
        )
        .expect("parse args");

        assert!(matches!(
            args,
            UpdateTaskArgs {
                update: TaskUpdate::NeverEnds { never_ends },
                ..
            } if never_ends
        ));
    }

    #[test]
    fn validate_args_trims_description_values() {
        let context = ToolContext::test();

        let (task_id, update) = parse_update_args(
            &json!({
                "field": "description",
                "description": "  updated task  "
            })
            .to_string(),
            &context,
        )
        .expect("validate args");

        assert_eq!(task_id, *context.task_id);
        assert_eq!(
            update,
            TaskUpdate::Description {
                description: "updated task".to_string(),
            }
        );
    }

    #[test]
    fn validate_args_rejects_empty_description() {
        let error = parse_update_args(
            &json!({
                "field": "description",
                "description": "   "
            })
            .to_string(),
            &ToolContext::test(),
        )
        .expect_err("empty description should fail");

        assert!(error.to_string().contains("description cannot be empty"));
    }

    #[test]
    fn parse_args_rejects_multiple_update_fields() {
        let error = parse_tool_args::<UpdateTaskArgs>(
            &json!({
                "field": "description",
                "description": "x",
                "never_ends": true
            })
            .to_string(),
        )
        .expect_err("multiple update fields should fail");

        assert!(error.to_string().contains("Invalid tool arguments"));
    }
}
