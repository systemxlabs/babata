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
}

impl OpenAIProvider {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    pub fn with_base_url(mut self, base_url: &str) -> Self {
        self.base_url = base_url.to_string();
        self
    }

    fn format_tools(&self, tools: &[ToolSpec]) -> Vec<ChatCompletionTool> {
        tools
            .iter()
            .map(|t| {
                ChatCompletionTool::Function(FunctionDefination {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    parameters: Some(t.parameters.clone()),
                    strict: None,
                })
            })
            .collect()
    }

    fn format_messages(
        &self,
        system_prompt: &str,
        messages: &[Message],
    ) -> BabataResult<Vec<Value>> {
        let mut json_messages = Vec::with_capacity(messages.len() + 1);

        let system_prompt = system_prompt.trim();
        if !system_prompt.is_empty() {
            json_messages.push(json!({
                "role": "system",
                "content": system_prompt
            }));
        }

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
        Ok(json_messages)
    }
}

#[async_trait::async_trait]
impl Provider for OpenAIProvider {
    fn name() -> &'static str {
        "openai"
    }

    fn supported_models() -> &'static [&'static str] {
        &["gpt-4.1"]
    }

    async fn generate<'a>(
        &self,
        request: GenerationReqest<'a>,
    ) -> BabataResult<GenerationResponse> {
        let mut body = json!({
            "model": request.model,
            "messages": self.format_messages(request.system_prompt, request.messages)?,
        });

        if !request.tools.is_empty() {
            body["tools"] = json!(self.format_tools(request.tools));
        }

        debug!(
            "Sending chat completions request to {}: {body}",
            self.base_url
        );

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                BabataError::provider(format!(
                    "Failed to send request to provider API ({}): {}",
                    self.base_url, e
                ))
            })?;

        // Check for errors
        let status = response.status();
        if response.status() != StatusCode::OK {
            let body = response.text().await.unwrap_or_default();
            return Err(BabataError::provider(format!(
                "Provider API ({}) returned error status {status}: {body}",
                self.base_url
            )));
        }

        let mut response_body: ChatCompletionResponse = response
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
            let mut parsed_calls = Vec::with_capacity(tool_calls.len());
            for tool_call in tool_calls {
                match tool_call {
                    ChatCompletionMessageToolCall::Function(function_tool_call) => {
                        parsed_calls.push(ToolCall {
                            call_id: function_tool_call.id.clone(),
                            tool_name: function_tool_call.function.name.clone(),
                            args: function_tool_call.function.arguments.clone(),
                        });
                    }
                }
            }

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
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatCompletionMessageParam>,
    pub tools: Option<Vec<ChatCompletionTool>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ChatCompletionMessageParam {
    Developer(ChatCompletionDeveloperMessageParam),
    User(ChatCompletionUserMessageParam),
    Assistant(ChatCompletionAssistantMessageParam),
    Tool(ChatCompletionToolMessageParam),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionDeveloperMessageParam {
    name: Option<String>,
    content: Vec<ChatCompletionContentPartText>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionUserMessageParam {
    pub name: Option<String>,
    pub content: Vec<ChatCompletionContentPart>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionAssistantMessageParam {
    pub name: Option<String>,
    pub content: Option<Vec<ChatCompletionContentPart>>,
    pub refusal: Option<String>,
    pub tool_calls: Option<Vec<ChatCompletionMessageToolCall>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionToolMessageParam {
    pub tool_call_id: String,
    pub content: Vec<ChatCompletionContentPartText>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatCompletionContentPart {
    Text(ChatCompletionContentPartText),
    ImageUrl(ChatCompletionContentPartImage),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionContentPartText {
    pub text: String,
    pub r#type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ChatCompletionContentPartImage {}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ChatCompletionTool {
    Function(FunctionDefination),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionDefination {
    pub name: String,
    pub description: String,
    pub parameters: Option<Value>,
    pub strict: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatCompletionChoice>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionChoice {
    pub index: u32,
    pub message: ChatCompletionMessage,
    pub finish_reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionMessage {
    pub role: ChatCompletionMessageRole,
    pub content: Option<String>,
    pub refusal: Option<String>,
    pub tool_calls: Option<Vec<ChatCompletionMessageToolCall>>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChatCompletionMessageRole {
    System,
    User,
    Assistant,
    Function,
    Tool,
    Developer,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ChatCompletionMessageToolCall {
    Function(ChatCompletionMessageFunctionToolCall),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionMessageFunctionToolCall {
    id: String,
    function: ChatCompletionsMessageToolCallFunction,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionsMessageToolCallFunction {
    pub name: String,
    pub arguments: String,
}
