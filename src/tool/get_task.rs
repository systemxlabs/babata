use reqwest::Client;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    BabataResult,
    error::BabataError,
    http::DEFAULT_HTTP_BASE_URL,
    tool::{Tool, ToolContext, ToolSpec},
};

#[derive(Debug)]
pub struct GetTaskTool {
    spec: ToolSpec,
    http_client: Client,
}

impl GetTaskTool {
    pub fn new() -> BabataResult<Self> {
        Ok(Self {
            spec: ToolSpec {
                name: "get_task".to_string(),
                description: "Get task metadata by id.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "task_id": {
                            "type": "string",
                            "description": "The UUID of the task to fetch"
                        }
                    },
                    "required": ["task_id"]
                }),
            },
            http_client: Client::new(),
        })
    }
}

#[async_trait::async_trait]
impl Tool for GetTaskTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, _context: &ToolContext<'_>) -> BabataResult<String> {
        let task_id = parse_args(args)?;

        let response = self
            .http_client
            .get(format!("{DEFAULT_HTTP_BASE_URL}/api/tasks/{task_id}"))
            .send()
            .await
            .map_err(|err| {
                BabataError::tool(format!("Failed to call get_task HTTP API: {}", err))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BabataError::tool(format!(
                "get_task HTTP API returned status {}: {}",
                status, body
            )));
        }

        response.text().await.map_err(|err| {
            BabataError::tool(format!(
                "Failed to read get_task HTTP API response body: {}",
                err
            ))
        })
    }
}

fn parse_args(args: &str) -> BabataResult<Uuid> {
    let args: Value = serde_json::from_str(args)?;
    let task_id = args["task_id"]
        .as_str()
        .ok_or_else(|| BabataError::tool("Missing required parameter: task_id"))?;

    Uuid::parse_str(task_id)
        .map_err(|err| BabataError::tool(format!("Invalid task_id '{}': {}", task_id, err)))
}
