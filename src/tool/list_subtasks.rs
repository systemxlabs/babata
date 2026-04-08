use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    BabataResult,
    task::TaskStore,
    tool::{Tool, ToolContext, ToolSpec, parse_tool_args},
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
                parameters: schemars::schema_for!(ListSubtasksArgs),
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
        let args: ListSubtasksArgs = parse_tool_args(args)?;
        let parent_task_id = args.parent_task_id.unwrap_or(*context.task_id);

        let tasks = self.task_store.list_subtasks(parent_task_id)?;
        serde_json::to_string(&tasks).map_err(Into::into)
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ListSubtasksArgs {
    #[schemars(description = "Optional UUID of the parent task to list subtasks for")]
    parent_task_id: Option<Uuid>,
}
