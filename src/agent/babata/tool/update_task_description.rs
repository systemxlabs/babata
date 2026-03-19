use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    BabataResult,
    agent::babata::{Tool, ToolContext, ToolSpec},
    error::BabataError,
    task::{TaskStatus, TaskStore},
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
                    "Update a task description in the local TaskStore for a running or paused task. If task_id is omitted, update the current task. Use this to keep task summaries accurate as work evolves."
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

        let task = self.task_store.get_task(task_id)?;
        if !matches!(task.status, TaskStatus::Running | TaskStatus::Paused) {
            return Err(BabataError::tool(format!(
                "Task '{}' cannot be updated from status '{}'; only running or paused tasks can be updated",
                task_id, task.status
            )));
        }

        self.task_store
            .update_task_description(task_id, description.clone())?;

        Ok(format!(
            "Updated description for task '{}': {}",
            task_id, description
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::TaskRecord;

    #[test]
    fn execute_rejects_non_running_non_paused_task() {
        let tool = UpdateTaskDescriptionTool::new().expect("tool");
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
}
