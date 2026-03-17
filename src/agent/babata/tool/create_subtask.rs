use reqwest::Client;
use serde_json::{Value, json};

use crate::{
    BabataResult,
    agent::babata::{Tool, ToolContext, ToolSpec},
    error::BabataError,
    http::DEFAULT_HTTP_BASE_URL,
};

#[derive(Debug)]
pub struct CreateSubtaskTool {
    spec: ToolSpec,
    http_client: Client,
}

impl CreateSubtaskTool {
    pub fn new() -> BabataResult<Self> {
        Ok(Self {
            spec: ToolSpec {
                name: "create_subtask".to_string(),
                description:
                    "Create a subtask for the current task through the local HTTP API. The current task is used as the parent task automatically. Supports an optional agent override."
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "prompt": {
                            "type": "string",
                            "description": "The prompt for the subtask to create"
                        },
                        "agent": {
                            "type": "string",
                            "description": "Optional agent name for the subtask"
                        }
                    },
                    "required": ["prompt"]
                }),
            },
            http_client: Client::new(),
        })
    }
}

#[async_trait::async_trait]
impl Tool for CreateSubtaskTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, context: &ToolContext<'_>) -> BabataResult<String> {
        let args: Value = serde_json::from_str(args)?;
        let prompt = args["prompt"]
            .as_str()
            .ok_or_else(|| BabataError::tool("Missing required parameter: prompt"))?;

        if prompt.trim().is_empty() {
            return Err(BabataError::tool("prompt cannot be empty"));
        }

        let request_body = json!({
            "prompt": prompt,
            "agent": args["agent"].as_str(),
            "parent_task_id": context.task_id,
        });

        let response = self
            .http_client
            .post(format!("{DEFAULT_HTTP_BASE_URL}/tasks"))
            .json(&request_body)
            .send()
            .await
            .map_err(|err| {
                BabataError::tool(format!("Failed to call create_subtask HTTP API: {}", err))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BabataError::tool(format!(
                "create_subtask HTTP API returned status {}: {}",
                status, body
            )));
        }

        response.text().await.map_err(|err| {
            BabataError::tool(format!(
                "Failed to read create_subtask HTTP API response body: {}",
                err
            ))
        })
    }
}
