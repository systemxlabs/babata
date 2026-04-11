use reqwest::Client;
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    BabataResult,
    error::BabataError,
    http::{SteerTaskRequest, http_base_url},
    message::Content,
    tool::{Tool, ToolContext, ToolSpec, parse_tool_args},
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
                parameters: schemars::schema_for!(SteerTaskArgs),
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
        let args: SteerTaskArgs = parse_tool_args(args)?;
        let base_url = http_base_url()?;
        let request = SteerTaskRequest {
            content: vec![Content::Text { text: args.content }],
        };

        let response = self
            .http_client
            .post(format!("{base_url}/api/tasks/{}/steer", args.task_id))
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
            args.task_id
        ))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SteerTaskArgs {
    #[schemars(description = "The UUID of the task to steer (must be running)")]
    task_id: Uuid,
    #[schemars(description = "The steering message content")]
    content: String,
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

        let params = serde_json::to_value(&spec.parameters).expect("serialize params");
        assert_eq!(params["type"], "object");
        assert_eq!(params["required"], json!(["task_id", "content"]));
    }

    #[test]
    fn parse_args_extracts_task_id_and_content() {
        let task_id = uuid::Uuid::new_v4();

        let args = parse_tool_args::<SteerTaskArgs>(
            &json!({
                "task_id": task_id,
                "content": "focus on tests",
            })
            .to_string(),
        )
        .expect("parse args");

        assert_eq!(args.task_id, task_id);
        assert_eq!(args.content, "focus on tests");
    }

    #[test]
    fn parse_args_allows_empty_content_string() {
        let args = parse_tool_args::<SteerTaskArgs>(
            &json!({
                "task_id": uuid::Uuid::new_v4(),
                "content": "   ",
            })
            .to_string(),
        )
        .expect("empty content still parses");

        assert_eq!(args.content, "   ");
    }
}
