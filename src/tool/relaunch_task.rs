use reqwest::Client;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    BabataResult,
    error::BabataError,
    http::{DEFAULT_HTTP_BASE_URL, RelaunchTaskRequest},
    tool::{Tool, ToolContext, ToolSpec},
};

#[derive(Debug)]
pub struct RelaunchTaskTool {
    spec: ToolSpec,
    http_client: Client,
}

impl RelaunchTaskTool {
    pub fn new() -> BabataResult<Self> {
        Ok(Self {
            spec: ToolSpec {
                name: "relaunch_task".to_string(),
                description:
                    "Relaunch a running task with a required reason. Use this when the task should continue with corrected instructions or after new information arrives."
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "task_id": {
                            "type": "string",
                            "description": "The UUID of the running task to relaunch"
                        },
                        "reason": {
                            "type": "string",
                            "description": "Why the task should be relaunched; this reason is injected into the relaunched task prompt"
                        }
                    },
                    "required": ["task_id", "reason"]
                }),
            },
            http_client: Client::new(),
        })
    }
}

#[async_trait::async_trait]
impl Tool for RelaunchTaskTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, _context: &ToolContext<'_>) -> BabataResult<String> {
        let args: Value = serde_json::from_str(args)?;
        let task_id = parse_task_id(&args)?;
        let reason = parse_reason(&args)?;

        let response = self
            .http_client
            .post(format!(
                "{DEFAULT_HTTP_BASE_URL}/api/tasks/{task_id}/relaunch"
            ))
            .json(&RelaunchTaskRequest {
                reason: reason.clone(),
            })
            .send()
            .await
            .map_err(|err| {
                BabataError::tool(format!("Failed to call relaunch_task HTTP API: {}", err))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BabataError::tool(format!(
                "relaunch_task HTTP API returned status {}: {}",
                status, body
            )));
        }

        Ok(format!(
            "Relaunched task '{}' with reason: {}",
            task_id, reason
        ))
    }
}

fn parse_task_id(args: &Value) -> BabataResult<Uuid> {
    let task_id = args["task_id"]
        .as_str()
        .ok_or_else(|| BabataError::tool("Missing required parameter: task_id"))?;
    Uuid::parse_str(task_id)
        .map_err(|err| BabataError::tool(format!("Invalid task_id '{}': {}", task_id, err)))
}

fn parse_reason(args: &Value) -> BabataResult<String> {
    let reason = args["reason"]
        .as_str()
        .ok_or_else(|| BabataError::tool("Missing required parameter: reason"))?
        .trim()
        .to_string();

    if reason.is_empty() {
        return Err(BabataError::tool("reason cannot be empty"));
    }

    Ok(reason)
}

#[cfg(test)]
mod tests {
    use super::parse_reason;
    use serde_json::json;

    #[test]
    fn parse_reason_rejects_empty_string() {
        let error = parse_reason(&json!({ "reason": "   " })).expect_err("empty reason");
        assert!(error.to_string().contains("reason cannot be empty"));
    }
}
