use schemars::JsonSchema;
use serde::Deserialize;
use std::path::PathBuf;

use crate::{
    BabataResult,
    memory::MessageStore,
    tool::{Tool, ToolContext, ToolSpec, parse_tool_args},
    utils::agent_dir,
};

const MAX_ROWS: usize = 100;

pub fn process_query_results_with_truncation<T: serde::Serialize>(
    results: &[T],
    context: &ToolContext<'_>,
    tool_name: &str,
) -> BabataResult<String> {
    if results.len() <= MAX_ROWS {
        return serde_json::to_string(results).map_err(crate::error::BabataError::from);
    }

    // Truncate: keep only the last MAX_ROWS rows
    let truncated_results = &results[results.len() - MAX_ROWS..];
    let truncated_json =
        serde_json::to_string(truncated_results).map_err(crate::error::BabataError::from)?;

    // Write full results to file
    let log_file_path = get_query_log_path(context, tool_name)?;
    let full_json =
        serde_json::to_string_pretty(results).map_err(crate::error::BabataError::from)?;
    std::fs::write(&log_file_path, full_json).map_err(|e| {
        crate::error::BabataError::internal(format!(
            "Failed to write {} query log to '{}': {}",
            tool_name,
            log_file_path.display(),
            e
        ))
    })?;

    let header = format!(
        "... (results truncated, showing last {} rows, full results written to {})\n",
        MAX_ROWS,
        log_file_path.display()
    );

    Ok(header + &truncated_json)
}

fn get_query_log_path(context: &ToolContext<'_>, tool_name: &str) -> BabataResult<PathBuf> {
    let task_dir = crate::utils::task_dir(*context.task_id)?;
    let log_file_name = format!("{}-call-{}.json", tool_name, context.call_id);
    Ok(task_dir.join(log_file_name))
}

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

    async fn execute(&self, args: &str, context: &ToolContext<'_>) -> BabataResult<String> {
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
        process_query_results_with_truncation(&results, context, "query_messages")
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_dir(context: &ToolContext) -> std::path::PathBuf {
        let dir = crate::utils::task_dir(*context.task_id).unwrap();
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_no_truncation_when_under_limit() {
        let context = ToolContext::test();
        let _ = setup_test_dir(&context);
        let results = vec![1, 2, 3];
        let output =
            process_query_results_with_truncation(&results, &context, "test_tool").unwrap();

        assert_eq!(output, "[1,2,3]");

        // Verify no file was created
        let log_path = get_query_log_path(&context, "test_tool").unwrap();
        assert!(!log_path.exists());
    }

    #[test]
    fn test_truncation_when_over_limit() {
        let context = ToolContext::test();
        let _ = setup_test_dir(&context);
        let results: Vec<i32> = (0..110).collect();
        let tool_name = "test_tool_trunc";

        let output = process_query_results_with_truncation(&results, &context, tool_name).unwrap();

        // Check header
        assert!(output.contains("results truncated"));
        assert!(output.contains("showing last 100 rows"));

        // Check truncated content (last 100: 10 to 109)
        let json_part = output.split('\n').next_back().unwrap();
        let decoded: Vec<i32> = serde_json::from_str(json_part).unwrap();
        assert_eq!(decoded.len(), 100);
        assert_eq!(decoded[0], 10);
        assert_eq!(decoded[99], 109);

        // Verify file creation and content
        let log_path = get_query_log_path(&context, tool_name).unwrap();
        assert!(log_path.exists());

        let file_content = fs::read_to_string(&log_path).unwrap();
        let full_results: Vec<i32> = serde_json::from_str(&file_content).unwrap();
        assert_eq!(full_results.len(), 110);
        assert_eq!(full_results[0], 0);
        assert_eq!(full_results[109], 109);

        // Cleanup
        let _ = fs::remove_file(log_path);
    }

    #[test]
    fn test_empty_results() {
        let context = ToolContext::test();
        let _ = setup_test_dir(&context);
        let results: Vec<i32> = vec![];
        let output =
            process_query_results_with_truncation(&results, &context, "test_tool_empty").unwrap();

        assert_eq!(output, "[]");

        let log_path = get_query_log_path(&context, "test_tool_empty").unwrap();
        assert!(!log_path.exists());
    }
}
