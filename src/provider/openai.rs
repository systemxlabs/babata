use log::{debug, warn};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
            .map(|t| ChatCompletionTool::Function {
                function: FunctionDefination {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    parameters: Some(t.parameters.clone()),
                    strict: None,
                },
            })
            .collect()
    }

    fn format_messages(
        &self,
        system_prompt: &str,
        messages: &[Message],
    ) -> BabataResult<Vec<ChatCompletionMessageParam>> {
        let mut request_messages = Vec::with_capacity(messages.len() + 1);

        let system_prompt = system_prompt.trim();
        if !system_prompt.is_empty() {
            request_messages.push(ChatCompletionMessageParam::System {
                content: system_prompt.to_string(),
            });
        }

        for message in messages {
            match message {
                Message::UserPrompt { content } => {
                    let parts = content
                        .iter()
                        .map(|part| match part {
                            Content::Text { text } => {
                                ChatCompletionContentPart::Text { text: text.clone() }
                            }
                            Content::ImageUrl { url } => ChatCompletionContentPart::ImageUrl {
                                image_url: ChatCompletionImageUrl { url: url.clone() },
                            },
                            Content::ImageData { data, media_type } => {
                                ChatCompletionContentPart::ImageUrl {
                                    image_url: ChatCompletionImageUrl {
                                        url: format!("data:{media_type};base64,{data}"),
                                    },
                                }
                            }
                        })
                        .collect::<Vec<_>>();

                    request_messages.push(ChatCompletionMessageParam::User { content: parts });
                }
                Message::AssistantToolCalls { calls } => {
                    let tool_calls = calls
                        .iter()
                        .map(|call| ChatCompletionMessageToolCall::Function {
                            id: call.call_id.clone(),
                            function: ChatCompletionsMessageToolCallFunction {
                                name: call.tool_name.clone(),
                                arguments: call.args.clone(),
                            },
                        })
                        .collect::<Vec<_>>();

                    request_messages.push(ChatCompletionMessageParam::Assistant {
                        content: None,
                        tool_calls: Some(tool_calls),
                    });
                }
                Message::AssistantResponse { content } => {
                    let mut parts = Vec::with_capacity(content.len());
                    for part in content {
                        match part {
                            Content::Text { text } => {
                                parts.push(ChatCompletionContentPart::Text { text: text.clone() })
                            }
                            Content::ImageUrl { .. } | Content::ImageData { .. } => {
                                warn!("OpenAI assistant responses do not support images yet");
                            }
                        }
                    }

                    request_messages.push(ChatCompletionMessageParam::Assistant {
                        content: Some(parts),
                        tool_calls: None,
                    });
                }
                Message::ToolResult { call, result } => {
                    request_messages.push(ChatCompletionMessageParam::Tool {
                        tool_call_id: call.call_id.clone(),
                        content: result.clone(),
                    })
                }
            }
        }

        Ok(request_messages)
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
        let request_body = ChatCompletionRequest {
            model: request.model.to_string(),
            messages: self.format_messages(request.system_prompt, request.messages)?,
            tools: (!request.tools.is_empty()).then(|| self.format_tools(request.tools)),
        };

        debug!(
            "Sending chat completions request to {}: {}",
            self.base_url,
            serde_json::to_string(&request_body)?
        );

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
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
                    ChatCompletionMessageToolCall::Function { id, function } => {
                        parsed_calls.push(ToolCall {
                            call_id: id,
                            tool_name: function.name,
                            args: function.arguments,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ChatCompletionTool>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum ChatCompletionMessageParam {
    System {
        content: String,
    },
    User {
        content: Vec<ChatCompletionContentPart>,
    },
    Assistant {
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<Vec<ChatCompletionContentPart>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_calls: Option<Vec<ChatCompletionMessageToolCall>>,
    },
    Tool {
        tool_call_id: String,
        content: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatCompletionContentPart {
    Text { text: String },
    ImageUrl { image_url: ChatCompletionImageUrl },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionImageUrl {
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ChatCompletionTool {
    Function { function: FunctionDefination },
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
    Function {
        id: String,
        function: ChatCompletionsMessageToolCallFunction,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionsMessageToolCallFunction {
    pub name: String,
    pub arguments: String,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::tool::ToolSpec;

    use super::OpenAIProvider;

    #[test]
    fn format_tools_uses_function_wrapper_shape() {
        let provider = OpenAIProvider::new("test-key");
        let tools = vec![ToolSpec {
            name: "read_file".to_string(),
            description: "Read file".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                },
                "required": ["path"]
            }),
        }];

        let payload =
            serde_json::to_value(provider.format_tools(&tools)).expect("serialize formatted tools");

        assert_eq!(payload[0]["type"], json!("function"));
        assert_eq!(payload[0]["function"]["name"], json!("read_file"));
        assert!(payload[0].get("name").is_none());
    }
}
