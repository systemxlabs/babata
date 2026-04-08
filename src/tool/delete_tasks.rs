use reqwest::Client;
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    BabataResult,
    error::BabataError,
    http::DEFAULT_HTTP_BASE_URL,
    tool::{Tool, ToolContext, ToolSpec, parse_tool_args},
};

#[derive(Debug)]
pub struct DeleteTasksTool {
    spec: ToolSpec,
    http_client: Client,
}

impl DeleteTasksTool {
    pub fn new() -> BabataResult<Self> {
        Ok(Self {
            spec: ToolSpec {
                name: "delete_tasks".to_string(),
                description: "Delete multiple tasks by their IDs. Each deletion is attempted and the result (success or failure) is returned for each task.".to_string(),
                parameters: schemars::schema_for!(DeleteTasksArgs),
            },
            http_client: Client::new(),
        })
    }
}

#[async_trait::async_trait]
impl Tool for DeleteTasksTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, _context: &ToolContext<'_>) -> BabataResult<String> {
        let args: DeleteTasksArgs = parse_tool_args(args)?;

        // Delete tasks
        let mut success_ids = Vec::new();
        let mut failures = Vec::new();

        for task_id in args.tasks {
            match delete_task(&self.http_client, task_id).await {
                Ok(_) => success_ids.push(task_id.to_string()),
                Err(err) => failures.push((task_id.to_string(), err.to_string())),
            }
        }

        // Format results as plain text with two sections
        let mut output = String::new();

        // Successful deletions
        output.push_str("Successfully deleted tasks:\n");
        if success_ids.is_empty() {
            output.push_str("  (none)\n");
        } else {
            for id in success_ids {
                output.push_str(&format!("  - {}\n", id));
            }
        }

        // Failed deletions
        output.push_str("\nFailed to delete tasks:\n");
        if failures.is_empty() {
            output.push_str("  (none)\n");
        } else {
            for (id, reason) in failures {
                output.push_str(&format!("  - {}: {}\n", id, reason));
            }
        }

        Ok(output)
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DeleteTasksArgs {
    #[schemars(description = "List of task IDs to delete")]
    tasks: Vec<Uuid>,
}

async fn delete_task(http_client: &Client, task_id: Uuid) -> BabataResult<String> {
    let url = format!("{DEFAULT_HTTP_BASE_URL}/api/tasks/{task_id}");

    let response = http_client.delete(&url).send().await.map_err(|err| {
        BabataError::tool(format!("Failed to call delete_task HTTP API: {}", err))
    })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(BabataError::tool(format!(
            "delete_task HTTP API returned status {}: {}",
            status, body
        )));
    }

    Ok(format!("Deleted task '{}'", task_id))
}
