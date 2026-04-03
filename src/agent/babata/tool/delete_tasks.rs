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

        // Delete tasks concurrently
        let mut results = Vec::with_capacity(task_ids.len());
        
        for task_id in task_ids {
            let result = delete_task(&self.http_client, task_id).await;
            results.push((task_id, result));
        }

        // Format results
        let output: Vec<Value> = results
            .into_iter()
            .map(|(task_id, result)| {
                match result {
                    Ok(msg) => json!({
                        "task_id": task_id.to_string(),
                        "success": true,
                        "message": msg
                    }),
                    Err(err) => json!({
                        "task_id": task_id.to_string(),
                        "success": false,
                        "error": err.to_string()
                    }),
                }
            })
            .collect();

        Ok(json!({ "results": output }).to_string())
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
                    Uuid::parse_str(s)
                        .map_err(|err| BabataError::tool(format!("Invalid task_id '{}': {}", s, err)))
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
