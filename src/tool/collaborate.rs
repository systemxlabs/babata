use reqwest::Client;
use schemars::JsonSchema;
use serde::Deserialize;
use std::time::Duration;
use tokio::time::sleep;

use crate::{
    BabataResult,
    error::BabataError,
    http::{CollaborateTaskRequest, DEFAULT_HTTP_BASE_URL},
    message::Content,
    task::CollaborationTaskState,
    tool::{Tool, ToolContext, ToolSpec, parse_tool_args},
};

const COLLABORATE_POLL_INTERVAL_MS: u64 = 200;

#[derive(Debug)]
pub struct CollaborateTool {
    spec: ToolSpec,
    http_client: Client,
}

impl CollaborateTool {
    pub fn new() -> Self {
        Self {
            spec: ToolSpec {
                name: "collaborate".to_string(),
                description:
                    "Ask another agent to collaborate on the current task. This waits until the collaborator finishes and returns that final response."
                        .to_string(),
                parameters: schemars::schema_for!(CollaborateArgs),
            },
            http_client: Client::new(),
        }
    }
}

impl Default for CollaborateTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for CollaborateTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, context: &ToolContext<'_>) -> BabataResult<String> {
        let args: CollaborateArgs = parse_tool_args(args)?;
        let request = CollaborateTaskRequest {
            agent: args.agent.clone(),
            prompt: args.prompt.clone(),
        };
        let response = self
            .http_client
            .post(format!(
                "{DEFAULT_HTTP_BASE_URL}/api/tasks/{}/collaborate",
                context.task_id
            ))
            .json(&request)
            .send()
            .await
            .map_err(|err| {
                BabataError::tool(format!("Failed to call collaborate HTTP API: {}", err))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BabataError::tool(format!(
                "collaborate HTTP API returned status {}: {}",
                status, body
            )));
        }

        loop {
            let response = self
                .http_client
                .get(format!(
                    "{DEFAULT_HTTP_BASE_URL}/api/tasks/{}/collaborate",
                    context.task_id
                ))
                .send()
                .await
                .map_err(|err| {
                    BabataError::tool(format!(
                        "Failed to query collaborate HTTP API status: {}",
                        err
                    ))
                })?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(BabataError::tool(format!(
                    "collaborate HTTP API status query returned status {}: {}",
                    status, body
                )));
            }

            let response = response
                .json::<CollaborationTaskState>()
                .await
                .map_err(|err| {
                    BabataError::tool(format!(
                        "Failed to deserialize collaborate HTTP API status response: {}",
                        err
                    ))
                })?;

            match response {
                CollaborationTaskState::NonExisting => {
                    return Err(BabataError::tool("Collaboration task is not existing"));
                }
                CollaborationTaskState::Running => {
                    sleep(Duration::from_millis(COLLABORATE_POLL_INTERVAL_MS)).await;
                }
                CollaborationTaskState::Succeed { result } => {
                    return render_collaboration_response(&args.agent, &result);
                }
                CollaborationTaskState::Failed { reason } => {
                    return Err(BabataError::tool(format!(
                        "Collaborator '{}' failed: {}",
                        args.agent, reason
                    )));
                }
            }
        }
    }
}

fn render_collaboration_response(agent: &str, response: &[Content]) -> BabataResult<String> {
    let rendered = match response {
        [Content::Text { text }] => text.clone(),
        _ => serde_json::to_string(response).map_err(|err| {
            BabataError::tool(format!(
                "Failed to serialize collaboration response from agent '{}': {}",
                agent, err
            ))
        })?,
    };

    Ok(format!("Collaborator '{}' response:\n{}", agent, rendered))
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CollaborateArgs {
    #[schemars(description = "The agent name to collaborate with on the current task")]
    agent: String,
    #[schemars(description = "What help or subproblem the collaborator should focus on")]
    prompt: String,
}

#[cfg(test)]
mod tests {
    use super::{CollaborateArgs, CollaborateTool, render_collaboration_response};
    use crate::{
        message::{Content, MediaType},
        tool::{Tool, parse_tool_args},
    };
    use serde_json::json;

    #[test]
    fn tool_spec_has_required_parameters() {
        let tool = CollaborateTool::new();
        let spec = tool.spec();

        assert_eq!(spec.name, "collaborate");
        let params = serde_json::to_value(&spec.parameters).expect("serialize params");
        assert_eq!(params["required"], json!(["agent", "prompt"]));
    }

    #[test]
    fn parse_args_extracts_agent_and_prompt() {
        let args = parse_tool_args::<CollaborateArgs>(
            &json!({
                "agent": "reviewer",
                "prompt": "check edge cases",
            })
            .to_string(),
        )
        .expect("parse args");

        assert_eq!(args.agent, "reviewer");
        assert_eq!(args.prompt, "check edge cases");
    }

    #[test]
    fn render_text_response_returns_plain_text() {
        let result = render_collaboration_response(
            "reviewer",
            &[Content::Text {
                text: "done".to_string(),
            }],
        )
        .expect("render");

        assert_eq!(result, "Collaborator 'reviewer' response:\ndone");
    }

    #[test]
    fn render_non_text_response_serializes_json() {
        let result = render_collaboration_response(
            "reviewer",
            &[Content::ImageData {
                data: "abc".to_string(),
                media_type: MediaType::ImagePng,
            }],
        )
        .expect("render");

        assert!(result.contains("\"type\":\"image_data\""));
    }
}
