use reqwest::Client;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    BabataResult,
    error::BabataError,
    http::{DEFAULT_HTTP_BASE_URL, SteerTaskRequest},
    message::Content,
    tool::{Tool, ToolContext, ToolSpec},
};

#[derive(Debug)]
pub struct SteerTaskTool {
    spec: ToolSpec,
    http_client: Client,
}

impl SteerTaskTool {
    pub fn new() -> Self {
        Self {
            spec: ToolSpec {
                name: "steer_task".to_string(),
                description: "Send a steering message to a running task to influence its behavior."
                    .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "task_id": {
                            "type": "string",
                            "description": "The UUID of the task to steer (must be running)"
                        },
                        "content": {
                            "type": "string",
                            "description": "The steering message content"
                        }
                    },
                    "required": ["task_id", "content"]
                }),
            },
            http_client: Client::new(),
        }
    }
}

impl Default for SteerTaskTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for SteerTaskTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, _context: &ToolContext<'_>) -> BabataResult<String> {
        let (task_id, content) = parse_args(args)?;
        let request = SteerTaskRequest {
            content: vec![Content::Text { text: content }],
        };

        let response = self
            .http_client
            .post(format!("{DEFAULT_HTTP_BASE_URL}/api/tasks/{task_id}/steer"))
            .json(&request)
            .send()
            .await
            .map_err(|err| {
                BabataError::tool(format!("Failed to call steer_task HTTP API: {}", err))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BabataError::tool(format!(
                "steer_task HTTP API returned status {}: {}",
                status, body
            )));
        }

        Ok(format!(
            "Steer message sent successfully to task {}",
            task_id
        ))
    }
}

fn parse_args(args: &str) -> BabataResult<(Uuid, String)> {
    let args: Value = serde_json::from_str(args)?;

    let task_id = args["task_id"]
        .as_str()
        .ok_or_else(|| BabataError::tool("Missing required parameter: task_id"))?;
    let content = args["content"]
        .as_str()
        .ok_or_else(|| BabataError::tool("Missing required parameter: content"))?;

    if content.trim().is_empty() {
        return Err(BabataError::tool("content cannot be empty"));
    }

    let task_id = task_id.parse::<uuid::Uuid>().map_err(|_| {
        BabataError::tool(format!(
            "Invalid task_id '{}' - expected a valid UUID",
            task_id
        ))
    })?;

    Ok((task_id, content.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tool_spec_has_required_parameters() {
        let tool = SteerTaskTool::new();
        let spec = tool.spec();

        assert_eq!(spec.name, "steer_task");
        assert!(!spec.description.is_empty());

        let params = &spec.parameters;
        assert_eq!(params["type"], "object");
        assert_eq!(params["required"], json!(["task_id", "content"]));
    }

    #[test]
    fn parse_args_extracts_task_id_and_content() {
        let task_id = uuid::Uuid::new_v4();

        let (parsed_task_id, content) = parse_args(
            &json!({
                "task_id": task_id,
                "content": "focus on tests",
            })
            .to_string(),
        )
        .expect("parse args");

        assert_eq!(parsed_task_id, task_id);
        assert_eq!(content, "focus on tests");
    }

    #[test]
    fn parse_args_rejects_empty_content() {
        let error = parse_args(
            &json!({
                "task_id": uuid::Uuid::new_v4(),
                "content": "   ",
            })
            .to_string(),
        )
        .expect_err("empty content should fail");

        assert!(error.to_string().contains("content cannot be empty"));
    }
}
