use log::{debug, warn};
use reqwest::Client;
use serde_json::{Value, json};

use crate::{
    BabataResult,
    error::BabataError,
    message::{Content, Message},
    provider::{
        GenerationReqest, GenerationResponse, InteractionRequest, InteractionResponse, Provider,
    },
    tool::ToolSpec,
};

#[derive(Debug)]
pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
}

impl OpenAIProvider {
    pub fn new(api_key: &str, base_url: &str, model: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
            model: model.to_string(),
        }
    }

    fn format_tools(&self, tools: &[ToolSpec]) -> Vec<Value> {
        tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters
                    }
                })
            })
            .collect()
    }

    fn format_messages(&self, messages: &[Message]) -> BabataResult<Vec<Value>> {
        let mut json_messages = Vec::with_capacity(messages.len());
        for message in messages {
            match message {
                Message::UserPrompt(contents) => {
                    let json_contents = contents
                        .iter()
                        .map(|content| match content {
                            Content::Text(text) => {
                                json!({
                                    "type": "text",
                                    "text": text
                                })
                            }
                            Content::ImageUrl(url) => {
                                json!({
                                    "type": "image_url",
                                    "image_url": json!({ "url": url })
                                })
                            }
                            Content::ImageData { data, media_type } => {
                                json!({
                                    "type": "image_url",
                                    "image_url": json!({

                                        "url": format!("data:{media_type};base64,{data}")
                                         })
                                })
                            }
                        })
                        .collect::<Vec<_>>();
                    json_messages.push(json!({
                        "role": "user",
                        "content": json_contents
                    }));
                }
                Message::AssistantToolCalls(calls) => {
                    json_messages.push(json!({
                        "role": "assistant",
                        "tool_calls": calls.iter().map(|call| {
                            json!({
                                "id": call.call_id,
                                "type": "function",
                                "function": json!({
                                    "name": call.tool_name,
                                    "arguments": call.args
                                }),
                            })
                        }).collect::<Vec<_>>()
                    }));
                }
                Message::AssistantResponse(contents) => {
                    let mut json_contents = Vec::with_capacity(contents.len());
                    for content in contents {
                        match content {
                            Content::Text(text) => {
                                json_contents.push(json!({
                                    "type": "text",
                                    "text": text
                                }));
                            }
                            Content::ImageUrl(_) | Content::ImageData { .. } => {
                                // This message might be created by other provider models
                                warn!("OpenAI assistant responses do not support images yet");
                            }
                        }
                    }
                    json_messages.push(json!({
                        "role": "assistant",
                        "content": json_contents
                    }));
                }
                Message::ToolResult { call, result } => json_messages.push(json!({
                    "role": "tool",
                    "tool_call_id": call.call_id,
                    "content": result
                })),
            }
        }
        // TODO push system messages
        Ok(json_messages)
    }
}

#[async_trait::async_trait]
impl Provider for OpenAIProvider {
    fn name(&self) -> &str {
        "OpenAI"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn generate<'a>(
        &self,
        request: GenerationReqest<'a>,
    ) -> BabataResult<GenerationResponse> {
        let mut body = json!({
            "model": self.model,
            "messages": self.format_messages(request.messages)?,
        });

        if !request.tools.is_empty() {
            body["tools"] = json!(self.format_tools(request.tools));
        }

        debug!("Sending OpenAI chat completions request: {body}");

        let _response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                BabataError::provider(format!("Failed to send request to OpenAI: {}", e))
            })?;

        todo!()
    }

    async fn interact(&self, _request: InteractionRequest) -> BabataResult<InteractionResponse> {
        todo!()
    }
}
