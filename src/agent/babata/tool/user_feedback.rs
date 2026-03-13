use serde_json::{Value, json};
use std::{collections::HashMap, sync::Arc};

use crate::{
    BabataResult,
    agent::babata::{Tool, ToolSpec},
    channel::Channel,
    error::BabataError,
    message::Content,
};

#[derive(Debug, Clone)]
pub struct UserFeedbackTool {
    spec: ToolSpec,
    channels: HashMap<String, Arc<dyn Channel>>,
}

impl UserFeedbackTool {
    pub fn new(channels: HashMap<String, Arc<dyn Channel>>) -> Self {
        Self {
            spec: ToolSpec {
                name: "user_feedback".to_string(),
                description:
                    "Ask the user a question through the configured channel and block until the user replies. Use this only when you need user input. Do not use it for notification-only messages."
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "The question to ask the user. This tool waits until the user replies."
                        },
                        "channel": {
                            "type": "string",
                            "description": "The channel name to use for asking the user"
                        }
                    },
                    "required": ["message", "channel"]
                }),
            },
            channels,
        }
    }
}

#[async_trait::async_trait]
impl Tool for UserFeedbackTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str) -> BabataResult<String> {
        let args: Value = serde_json::from_str(args)?;
        let message = args["message"]
            .as_str()
            .ok_or_else(|| BabataError::tool("Missing required parameter: message"))?;
        let channel_name = args["channel"]
            .as_str()
            .ok_or_else(|| BabataError::tool("Missing required parameter: channel"))?;

        if message.trim().is_empty() {
            return Err(BabataError::tool("message cannot be empty"));
        }

        let channel = self
            .channels
            .get(channel_name)
            .ok_or_else(|| BabataError::tool(format!("Channel '{}' not found", channel_name)))?;

        let response = channel
            .feedback(vec![Content::Text {
                text: format!("[Ask Feedback] {message}"),
            }])
            .await?;

        serde_json::to_string(&response).map_err(|err| {
            BabataError::tool(format!(
                "Failed to serialize user feedback content into JSON: {}",
                err
            ))
        })
    }
}
