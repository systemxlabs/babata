use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    BabataResult,
    task::TaskStore,
    tool::{Tool, ToolContext, ToolSpec, parse_tool_args},
};

#[derive(Debug)]
pub struct QueryTasksTool {
    spec: ToolSpec,
    task_store: TaskStore,
}

impl QueryTasksTool {
    pub fn new() -> BabataResult<Self> {
        Ok(Self {
            spec: ToolSpec {
                name: "query_tasks".to_string(),
                description: "Query tasks from the task store using a SQL SELECT statement. \
                    Returns each row as a JSON object. \
                    The tasks table has columns: task_id, description, agent, status, \
                    parent_task_id, root_task_id, created_at, never_ends."
                    .to_string(),
                parameters: schemars::schema_for!(QueryTasksArgs),
            },
            task_store: TaskStore::new()?,
        })
    }
}

#[async_trait::async_trait]
impl Tool for QueryTasksTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, context: &ToolContext<'_>) -> BabataResult<String> {
        let QueryTasksArgs { sql } = parse_tool_args(args)?;

        crate::tool::query_messages::validate_select_query(&sql)?;

        let results = self.task_store.query_sql(&sql)?;
        crate::tool::query_messages::process_query_results_with_truncation(
            &results,
            context,
            "query_tasks",
        )
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct QueryTasksArgs {
    #[schemars(description = "SQL SELECT query to execute against the tasks table")]
    sql: String,
}
