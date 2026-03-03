use log::{debug, warn};
use reqwest::{Client, StatusCode, header::USER_AGENT};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    BabataResult,
    error::BabataError,
    message::{Content, Message, ToolCall},
    provider::{GenerationReqest, GenerationResponse, InteractionRequest, InteractionResponse},
    tool::ToolSpec,
};

#[derive(Debug)]
pub struct OpenAICompatibleProvider {
    client: Client,
    api_key: String,
    base_url: String,
    user_agent: Option<String>,
}

impl OpenAICompatibleProvider {
    pub fn new(api_key: &str, base_url: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
            user_agent: None,
        }
    }

    pub fn with_user_agent(mut self, user_agent: Option<String>) -> Self {
        self.user_agent = user_agent;
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
                                image_url: ChatCompletionContentPartImage { url: url.clone() },
                            },
                            Content::ImageData { data, media_type } => {
                                ChatCompletionContentPart::ImageUrl {
                                    image_url: ChatCompletionContentPartImage {
                                        url: format!("data:{media_type};base64,{data}"),
                                    },
                                }
                            }
                        })
                        .collect::<Vec<_>>();

                    request_messages.push(ChatCompletionMessageParam::User { content: parts });
                }
                Message::AssistantToolCalls {
                    calls,
                    reasoning_content,
                } => {
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
                        reasoning_content: reasoning_content.clone(),
                        tool_calls: Some(tool_calls),
                    });
                }
                Message::AssistantResponse {
                    content,
                    reasoning_content,
                } => {
                    let mut parts = Vec::with_capacity(content.len());
                    for part in content {
                        match part {
                            Content::Text { text } => {
                                parts.push(ChatCompletionContentPart::Text { text: text.clone() })
                            }
                            Content::ImageUrl { .. } | Content::ImageData { .. } => {
                                warn!(
                                    "OpenAI-compatible assistant responses do not support images yet"
                                );
                            }
                        }
                    }

                    request_messages.push(ChatCompletionMessageParam::Assistant {
                        content: Some(parts),
                        reasoning_content: reasoning_content.clone(),
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

    pub async fn generate<'a>(
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
            serde_json::to_string_pretty(&request_body)?
        );

        let mut request_builder = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json");

        if let Some(user_agent) = &self.user_agent {
            request_builder = request_builder.header(USER_AGENT, user_agent);
        }

        let response = request_builder
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                BabataError::provider(format!(
                    "Failed to send request to provider API ({}): {}",
                    self.base_url, e
                ))
            })?;

        let status = response.status();
        if status != StatusCode::OK {
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
            "OpenAI-compatible response: {}",
            serde_json::to_string_pretty(&response_body)?
        );

        if response_body.choices.is_empty() {
            return Err(BabataError::provider("No choices in response"));
        }

        let choice = response_body.choices.remove(0);

        if let Some(tool_calls) = choice.message.tool_calls {
            let mut parsed_calls = Vec::with_capacity(tool_calls.len());
            for tool_call in tool_calls {
                let ChatCompletionMessageToolCall::Function { id, function } = tool_call;
                parsed_calls.push(ToolCall {
                    call_id: id,
                    tool_name: function.name,
                    args: function.arguments,
                });
            }

            if !parsed_calls.is_empty() {
                return Ok(GenerationResponse {
                    message: Message::AssistantToolCalls {
                        calls: parsed_calls,
                        reasoning_content: choice.message.reasoning_content,
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
                reasoning_content: choice.message.reasoning_content,
            },
        })
    }

    pub async fn interact(
        &self,
        _request: InteractionRequest,
    ) -> BabataResult<InteractionResponse> {
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
        reasoning_content: Option<String>,
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
    ImageUrl { image_url: ChatCompletionContentPartImage },
    InputAudio { input_audio: ChatCompletionContentPartInputAudio },
    File { file: ChatCompletionContentPartFile },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionContentPartImage {
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionContentPartInputAudio {
    pub data: String,
    pub format: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionContentPartFile {
    pub file_data: Option<String>,
    pub file_id: Option<String>,
    pub filename: Option<String>,
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
    pub reasoning_content: Option<String>,
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

    use super::OpenAICompatibleProvider;

    #[test]
    fn format_tools_uses_function_wrapper_shape() {
        let provider = OpenAICompatibleProvider::new("test-key", "https://example.com/v1");
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
