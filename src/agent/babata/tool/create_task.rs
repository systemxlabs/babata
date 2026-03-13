use reqwest::Client;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    BabataResult,
    agent::babata::{Tool, ToolSpec},
    error::BabataError,
    http::DEFAULT_HTTP_BASE_URL,
};

#[derive(Debug)]
pub struct CreateTaskTool {
    spec: ToolSpec,
    http_client: Client,
}

impl CreateTaskTool {
    pub fn new() -> BabataResult<Self> {
        Ok(Self {
            spec: ToolSpec {
                name: "create_task".to_string(),
                description:
                    "Create a new task through the local HTTP API. Supports optional agent and parent_task_id."
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "prompt": {
                            "type": "string",
                            "description": "The task prompt text"
                        },
                        "agent": {
                            "type": "string",
                            "description": "Optional agent name"
                        },
                        "parent_task_id": {
                            "type": "string",
                            "description": "Optional parent task UUID"
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
impl Tool for CreateTaskTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str) -> BabataResult<String> {
        let args: Value = serde_json::from_str(args)?;
        let prompt = args["prompt"]
            .as_str()
            .ok_or_else(|| BabataError::tool("Missing required parameter: prompt"))?;

        if prompt.trim().is_empty() {
            return Err(BabataError::tool("prompt cannot be empty"));
        }

        if let Some(parent_task_id) = args["parent_task_id"].as_str() {
            Uuid::parse_str(parent_task_id).map_err(|err| {
                BabataError::tool(format!(
                    "Invalid parent_task_id '{}': {}",
                    parent_task_id, err
                ))
            })?;
        }

        let request_body = json!({
            "prompt": prompt,
            "agent": args["agent"].as_str(),
            "parent_task_id": args["parent_task_id"].as_str(),
        });

        let response = self
            .http_client
            .post(format!("{DEFAULT_HTTP_BASE_URL}/tasks"))
            .json(&request_body)
            .send()
            .await
            .map_err(|err| {
                BabataError::tool(format!("Failed to call create_task HTTP API: {}", err))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BabataError::tool(format!(
                "create_task HTTP API returned status {}: {}",
                status, body
            )));
        }

        response.text().await.map_err(|err| {
            BabataError::tool(format!(
                "Failed to read create_task HTTP API response body: {}",
                err
            ))
        })
    }
}
