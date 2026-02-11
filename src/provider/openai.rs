use log::{debug, warn};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    BabataResult,
    error::BabataError,
    message::{Content, Message, ToolCall},
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
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: model.to_string(),
        }
    }

    pub fn with_base_url(mut self, base_url: &str) -> Self {
        self.base_url = base_url.to_string();
        self
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
                Message::UserPrompt { content } => {
                    let json_contents = content
                        .iter()
                        .map(|content| match content {
                            Content::Text { text } => {
                                json!({
                                    "type": "text",
                                    "text": text
                                })
                            }
                            Content::ImageUrl { url } => {
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
                Message::AssistantToolCalls { calls } => {
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
                Message::AssistantResponse { content } => {
                    let mut json_contents = Vec::with_capacity(content.len());
                    for part in content {
                        match part {
                            Content::Text { text } => {
                                json_contents.push(json!({
                                    "type": "text",
                                    "text": text
                                }));
                            }
                            Content::ImageUrl { .. } | Content::ImageData { .. } => {
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
    fn name() -> &'static str {
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

        let response = self
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

        // Check for errors
        let status = response.status();
        if response.status() != StatusCode::OK {
            let body = response.text().await.unwrap_or_default();
            return Err(BabataError::provider(format!(
                "OpenAI API returned error status {status}: {body}",
            )));
        }

        let mut response_body: ChatCompletionsResponse = response
            .json()
            .await
            .map_err(|e| BabataError::provider(format!("Failed to parse response body: {e}")))?;
        debug!(
            "OpenAI response: {}",
            serde_json::to_string_pretty(&response_body)?
        );

        if response_body.choices.is_empty() {
            return Err(BabataError::provider("No choices in response"));
        }

        let choice = response_body.choices.remove(0);

        // Check for tool calls
        if let Some(tool_calls) = choice.message.tool_calls {
            let parsed_calls: Vec<ToolCall> = tool_calls
                .iter()
                .map(|tc| ToolCall {
                    call_id: tc.id.clone(),
                    tool_name: tc.function.name.clone(),
                    args: tc.function.arguments.clone(),
                })
                .collect();

            if !parsed_calls.is_empty() {
                return Ok(GenerationResponse {
                    message: Message::AssistantToolCalls {
                        calls: parsed_calls,
                    },
                });
            }
        }

        let Some(content) = choice.message.content else {
            return Err(BabataError::provider("No content in assistant message"));
        };

        Ok(GenerationResponse {
            message: Message::AssistantResponse {
                content: vec![Content::Text { text: content }],
            },
        })
    }

    async fn interact(&self, _request: InteractionRequest) -> BabataResult<InteractionResponse> {
        todo!()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionsResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatCompletionChoice>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionChoice {
    pub index: u32,
    pub message: ChatCompletionsMessage,
    pub finish_reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionsMessage {
    pub role: String,
    pub content: Option<String>,
    pub refusal: Option<String>,
    pub tool_calls: Option<Vec<ChatCompletionsMessageToolCall>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionsMessageToolCall {
    pub id: String,
    pub r#type: String,
    pub function: ChatCompletionsMessageToolCallFunction,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionsMessageToolCallFunction {
    pub name: String,
    pub arguments: Value,
}
