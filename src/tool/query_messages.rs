use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    BabataResult,
    memory::MessageStore,
    tool::{Tool, ToolContext, ToolSpec, parse_tool_args},
    utils::agent_dir,
};

#[derive(Debug)]
pub struct QueryMessagesTool {
    spec: ToolSpec,
}

impl Default for QueryMessagesTool {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryMessagesTool {
    pub fn new() -> Self {
        Self {
            spec: ToolSpec {
                name: "query_messages".to_string(),
                description: "Query messages from the message store using a SQL SELECT statement. \
                    Returns each row as a JSON object. \
                    The messages table has columns: task_id, message_type, content, \
                    signature, tool_calls, result, created_at."
                    .to_string(),
                parameters: schemars::schema_for!(QueryMessagesArgs),
            },
        }
    }
}

#[async_trait::async_trait]
impl Tool for QueryMessagesTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, _context: &ToolContext<'_>) -> BabataResult<String> {
        let QueryMessagesArgs { agent, sql } = parse_tool_args(args)?;

        // Basic validation to ensure it's a SELECT query
        let trimmed = sql.trim().to_uppercase();
        if !trimmed.starts_with("SELECT") {
            return Err(crate::error::BabataError::tool(
                "Only SELECT queries are allowed".to_string(),
            ));
        }

        let agent_home = agent_dir(&agent)?;
        let store = MessageStore::new(&agent_home)?;
        let results = store.query_sql(&sql)?;
        serde_json::to_string(&results).map_err(Into::into)
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct QueryMessagesArgs {
    #[schemars(description = "The agent name whose message database to query")]
    agent: String,
    #[schemars(description = "SQL SELECT query to execute against the messages table")]
    sql: String,
}
