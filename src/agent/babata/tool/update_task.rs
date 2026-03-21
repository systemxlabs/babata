use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    BabataResult,
    agent::babata::{Tool, ToolContext, ToolSpec},
    error::BabataError,
    task::{TaskStatus, TaskStore},
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
                    "Update task fields in the local TaskStore for a running or paused task. If task_id is omitted, update the current task. Use this to keep task summaries and never_ends flags accurate as work evolves."
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
        let args: Value = serde_json::from_str(args)?;
        let task_id = match args["task_id"].as_str() {
            Some(task_id) => Uuid::parse_str(task_id).map_err(|err| {
                BabataError::tool(format!("Invalid task_id '{}': {}", task_id, err))
            })?,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::TaskRecord;

    #[test]
    fn execute_rejects_non_running_non_paused_task() {
        let tool = UpdateTaskTool::new().expect("tool");
        let task_id = Uuid::new_v4();
        tool.task_store
            .insert_task(TaskRecord {
                task_id,
                description: "before".to_string(),
                agent: None,
                status: TaskStatus::Done,
                parent_task_id: None,
                root_task_id: task_id,
                created_at: 0,
                never_ends: false,
            })
            .expect("insert task");

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let error = runtime
            .block_on(
                tool.execute(
                    &json!({
                        "task_id": task_id.to_string(),
                        "description": "after"
                    })
                    .to_string(),
                    &ToolContext::test(),
                ),
            )
            .expect_err("done task should fail");

        assert!(error.to_string().contains("only running or paused tasks"));
    }

    #[test]
    fn execute_updates_description_and_never_ends() {
        let tool = UpdateTaskTool::new().expect("tool");
        let task_id = Uuid::new_v4();
        tool.task_store
            .insert_task(TaskRecord {
                task_id,
                description: "before".to_string(),
                agent: None,
                status: TaskStatus::Running,
                parent_task_id: None,
                root_task_id: task_id,
                created_at: 0,
                never_ends: false,
            })
            .expect("insert task");

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let result = runtime
            .block_on(
                tool.execute(
                    &json!({
                        "task_id": task_id.to_string(),
                        "description": "after",
                        "never_ends": true
                    })
                    .to_string(),
                    &ToolContext::test(),
                ),
            )
            .expect("running task should update");

        assert!(result.contains("description='after'"));
        assert!(result.contains("never_ends=true"));

        let task = tool.task_store.get_task(task_id).expect("updated task");
        assert_eq!(task.description, "after");
        assert!(task.never_ends);
    }

    #[test]
    fn execute_requires_at_least_one_update_field() {
        let tool = UpdateTaskTool::new().expect("tool");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let error = runtime
            .block_on(tool.execute("{}", &ToolContext::test()))
            .expect_err("missing update fields should fail");

        assert!(
            error
                .to_string()
                .contains("At least one of description or never_ends must be provided")
        );
    }
}
