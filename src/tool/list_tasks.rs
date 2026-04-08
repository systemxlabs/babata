use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    BabataResult,
    http::ListTasksResponse,
    task::{TaskStatus, TaskStore},
    tool::{Tool, ToolContext, ToolSpec, parse_tool_args},
};

#[derive(Debug)]
pub struct ListTasksTool {
    spec: ToolSpec,
    task_store: TaskStore,
}

impl ListTasksTool {
    pub fn new() -> BabataResult<Self> {
        Ok(Self {
            spec: ToolSpec {
                name: "list_tasks".to_string(),
                description:
                    "List tasks. Supports optional status filter and offset. The limit parameter is required."
                        .to_string(),
                parameters: schemars::schema_for!(ListTasksArgs),
            },
            task_store: TaskStore::new()?,
        })
    }
}

#[async_trait::async_trait]
impl Tool for ListTasksTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, _context: &ToolContext<'_>) -> BabataResult<String> {
        let ListTasksArgs {
            status,
            limit,
            offset,
        } = parse_tool_args(args)?;

        let tasks = self.task_store.list_tasks(status, limit, offset)?;
        let response = ListTasksResponse::from_records(tasks);
        serde_json::to_string(&response).map_err(Into::into)
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ListTasksArgs {
    #[schemars(description = "Optional task status filter")]
    status: Option<TaskStatus>,
    #[schemars(description = "Required max number of tasks to return")]
    limit: usize,
    #[schemars(description = "Optional number of tasks to skip before returning results")]
    offset: Option<usize>,
}
