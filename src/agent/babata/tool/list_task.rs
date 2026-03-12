use reqwest::Client;
use serde_json::{Value, json};

use crate::{
    BabataResult,
    agent::babata::{Tool, ToolSpec},
    error::BabataError,
    http::DEFAULT_HTTP_BASE_URL,
};

#[derive(Debug)]
pub struct ListTaskTool {
    spec: ToolSpec,
    http_client: Client,
}

impl ListTaskTool {
    pub fn new() -> BabataResult<Self> {
        Ok(Self {
            spec: ToolSpec {
                name: "list_task".to_string(),
                description:
                    "List tasks through the local HTTP API. Supports optional status filter and limit."
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "status": {
                            "type": "string",
                            "description": "Optional task status filter"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Optional max number of tasks to return"
                        }
                    }
                }),
            },
            http_client: Client::new(),
        })
    }
}

#[async_trait::async_trait]
impl Tool for ListTaskTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str) -> BabataResult<String> {
        let args: Value = serde_json::from_str(args)?;
        let status = args["status"].as_str();
        let limit = args["limit"].as_u64();

        let mut request = self.http_client.get(format!("{DEFAULT_HTTP_BASE_URL}/tasks"));
        if let Some(status) = status {
            request = request.query(&[("status", status)]);
        }
        if let Some(limit) = limit {
            request = request.query(&[("limit", limit)]);
        }

        let response = request.send().await.map_err(|err| {
            BabataError::tool(format!("Failed to call list_task HTTP API: {}", err))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BabataError::tool(format!(
                "list_task HTTP API returned status {}: {}",
                status, body
            )));
        }

        response.text().await.map_err(|err| {
            BabataError::tool(format!(
                "Failed to read list_task HTTP API response body: {}",
                err
            ))
        })
    }
}
