use reqwest::Client;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    BabataResult,
    agent::babata::{Tool, ToolContext, ToolSpec},
    error::BabataError,
    http::DEFAULT_HTTP_BASE_URL,
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
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "tasks": {
                            "type": "array",
                            "description": "List of task IDs to delete",
                            "items": {
                                "type": "string",
                                "description": "The UUID of the task to delete"
                            }
                        }
                    },
                    "required": ["tasks"]
                }),
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
        let task_ids = parse_args(args)?;

        // Delete tasks
        let mut success_ids = Vec::new();
        let mut failures = Vec::new();

        for task_id in task_ids {
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

fn parse_args(args: &str) -> BabataResult<Vec<Uuid>> {
    let args: Value = serde_json::from_str(args)?;
    let tasks = args["tasks"]
        .as_array()
        .ok_or_else(|| BabataError::tool("Missing required parameter: tasks (array of task IDs)"))?
        .iter()
        .map(|v| {
            v.as_str()
                .ok_or_else(|| BabataError::tool("Each task_id must be a string"))
                .and_then(|s| {
                    Uuid::parse_str(s).map_err(|err| {
                        BabataError::tool(format!("Invalid task_id '{}': {}", s, err))
                    })
                })
        })
        .collect::<BabataResult<Vec<_>>>()?;

    Ok(tasks)
}

async fn delete_task(http_client: &Client, task_id: Uuid) -> BabataResult<String> {
    let url = format!("{DEFAULT_HTTP_BASE_URL}/tasks/{task_id}");

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
