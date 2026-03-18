use serde_json::json;

use crate::{
    BabataResult,
    agent::babata::{Tool, ToolContext, ToolSpec},
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
                description:
                    "List direct subtasks of the current task by querying the local TaskStore."
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
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

    async fn execute(&self, _args: &str, context: &ToolContext<'_>) -> BabataResult<String> {
        let tasks = self.task_store.list_subtasks(*context.task_id)?;
        serde_json::to_string(&tasks).map_err(Into::into)
    }
}
