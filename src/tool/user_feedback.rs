use schemars::JsonSchema;
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};

use crate::{
    BabataResult,
    channel::Channel,
    error::BabataError,
    message::Content,
    tool::{Tool, ToolContext, ToolSpec, parse_tool_args},
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
                parameters: schemars::schema_for!(UserFeedbackArgs),
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

    async fn execute(&self, args: &str, _context: &ToolContext<'_>) -> BabataResult<String> {
        let args: UserFeedbackArgs = parse_tool_args(args)?;

        let channel = self
            .channels
            .get(&args.channel)
            .or_else(|| {
                self.channels
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case(&args.channel))
                    .map(|(_, channel)| channel)
            })
            .ok_or_else(|| BabataError::tool(format!("Channel '{}' not found", args.channel)))?;

        let response = channel
            .feedback(vec![Content::Text {
                text: format!("[Ask Feedback] {}", args.message),
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

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct UserFeedbackArgs {
    #[schemars(
        description = "The question to ask the user. This tool waits until the user replies."
    )]
    message: String,
    #[schemars(description = "The channel name to use for asking the user")]
    channel: String,
}
