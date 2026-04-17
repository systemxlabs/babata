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
        let (task_id, update) = validate_update_args(args, context)?;

        let task = self.task_store.get_task(task_id)?;
        if !matches!(task.status, TaskStatus::Running | TaskStatus::Paused) {
            return Err(BabataError::tool(format!(
                "Task '{}' cannot be updated from status '{}'; only running or paused tasks can be updated",
                task_id, task.status
            )));
        }

        self.task_store.update_task(task_id, update.clone())?;

        let update_description = match &update {
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
#[serde(deny_unknown_fields)]
struct UpdateTaskArgs {
    #[schemars(
        description = "Optional UUID of the task to update. If omitted, the current task is used."
    )]
    task_id: Option<Uuid>,
    #[schemars(description = "Task field to update: 'description' or 'never_ends'")]
    field: UpdateTaskField,
    #[schemars(description = "New task description when field is 'description'")]
    description: Option<String>,
    #[schemars(
        description = "New value for the task's never_ends flag when field is 'never_ends'"
    )]
    never_ends: Option<bool>,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum UpdateTaskField {
    Description,
    NeverEnds,
}

fn validate_update_args(
    args: UpdateTaskArgs,
    context: &ToolContext<'_>,
) -> BabataResult<(Uuid, TaskUpdate)> {
    let task_id = args.task_id.unwrap_or(*context.task_id);

    let update = match args.field {
        UpdateTaskField::Description => {
            if args.never_ends.is_some() {
                return Err(BabataError::tool(
                    "never_ends must not be provided when field is 'description'",
                ));
            }

            let description = args
                .description
                .ok_or_else(|| {
                    BabataError::tool("description is required when field is 'description'")
                })?
                .trim()
                .to_string();

            if description.is_empty() {
                return Err(BabataError::tool("description cannot be empty"));
            }

            TaskUpdate::Description { description }
        }
        UpdateTaskField::NeverEnds => {
            if args.description.is_some() {
                return Err(BabataError::tool(
                    "description must not be provided when field is 'never_ends'",
                ));
            }

            let never_ends = args.never_ends.ok_or_else(|| {
                BabataError::tool("never_ends is required when field is 'never_ends'")
            })?;

            TaskUpdate::NeverEnds { never_ends }
        }
    };

    Ok((task_id, update))
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
        super::validate_update_args(args, context)
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
                field: super::UpdateTaskField::Description,
                description: Some(description),
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
                field: super::UpdateTaskField::NeverEnds,
                never_ends: Some(never_ends),
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
        let error = parse_update_args(
            &json!({
                "field": "description",
                "description": "x",
                "never_ends": true
            })
            .to_string(),
            &ToolContext::test(),
        )
        .expect_err("multiple update fields should fail");

        assert!(
            error
                .to_string()
                .contains("never_ends must not be provided")
        );
    }

    #[test]
    fn parse_args_rejects_missing_description_for_description_field() {
        let error = parse_update_args(
            &json!({
                "field": "description"
            })
            .to_string(),
            &ToolContext::test(),
        )
        .expect_err("missing description should fail");

        assert!(error.to_string().contains("description is required"));
    }

    #[test]
    fn parse_args_rejects_missing_never_ends_for_never_ends_field() {
        let error = parse_update_args(
            &json!({
                "field": "never_ends"
            })
            .to_string(),
            &ToolContext::test(),
        )
        .expect_err("missing never_ends should fail");

        assert!(error.to_string().contains("never_ends is required"));
    }
}
