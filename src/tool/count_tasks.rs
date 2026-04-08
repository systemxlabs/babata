use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    BabataResult,
    task::{TaskStatus, TaskStore},
    tool::{Tool, ToolContext, ToolSpec, parse_tool_args},
};

#[derive(Debug)]
pub struct CountTasksTool {
    spec: ToolSpec,
    task_store: TaskStore,
}

impl CountTasksTool {
    pub fn new() -> BabataResult<Self> {
        Ok(Self {
            spec: ToolSpec {
                name: "count_tasks".to_string(),
                description: "Count tasks. Supports optional status filter.".to_string(),
                parameters: schemars::schema_for!(CountTasksArgs),
            },
            task_store: TaskStore::new()?,
        })
    }
}

#[async_trait::async_trait]
impl Tool for CountTasksTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, _context: &ToolContext<'_>) -> BabataResult<String> {
        let args: CountTasksArgs = parse_tool_args(args)?;

        let count = self.task_store.count_tasks(args.status)?;
        Ok(count.to_string())
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CountTasksArgs {
    #[schemars(description = "Optional task status filter")]
    status: Option<TaskStatus>,
}
