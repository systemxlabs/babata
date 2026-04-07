use log::{debug, warn};
use reqwest::{Client, StatusCode, header::USER_AGENT};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    BabataResult,
    error::BabataError,
    message::{Content, MediaType, Message, ToolCall},
    provider::{GenerationRequest, GenerationResponse, InteractionRequest, InteractionResponse},
    tool::ToolSpec,
};

#[derive(Debug)]
pub struct OpenAICompatibleProvider {
    client: Client,
    api_key: String,
    base_url: String,
    user_agent: Option<String>,
    combine_system_prompt: bool,
}

impl OpenAICompatibleProvider {
    pub fn new(api_key: &str, base_url: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
            user_agent: None,
            combine_system_prompt: false,
        }
    }

    pub fn with_user_agent(mut self, user_agent: Option<String>) -> Self {
        self.user_agent = user_agent;
        self
    }

    pub fn with_combined_system_prompt(mut self, combine_system_prompt: bool) -> Self {
        self.combine_system_prompt = combine_system_prompt;
        self
    }

    fn format_tools(&self, tools: &[ToolSpec]) -> Vec<ChatCompletionTool> {
        tools
            .iter()
            .map(|t| ChatCompletionTool::Function {
                function: FunctionDefinition {
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
        system_prompts: &[String],
        context: &str,
        prompts: &[Message],
    ) -> BabataResult<Vec<ChatCompletionMessageParam>> {
        let mut request_messages = Vec::with_capacity(prompts.len() + system_prompts.len() + 1);

        if self.combine_system_prompt {
            if let Some(content) = build_combined_system_prompt(system_prompts, context) {
                request_messages.push(ChatCompletionMessageParam::System { content });
            }
        } else {
            for system_prompt in system_prompts {
                if system_prompt.is_empty() {
                    continue;
                }
                request_messages.push(ChatCompletionMessageParam::System {
                    content: system_prompt.to_string(),
                });
            }

            if !context.trim().is_empty() {
                request_messages.push(ChatCompletionMessageParam::System {
                    content: format!("Context:\n{context}"),
                });
            }
        }

        for message in prompts {
            match message {
                Message::UserPrompt { content, .. } | Message::UserSteering { content, .. } => {
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
                                        url: format!(
                                            "data:{};base64,{data}",
                                            media_type.as_mime_str()
                                        ),
                                    },
                                }
                            }
                            Content::AudioData { data, media_type } => {
                                ChatCompletionContentPart::InputAudio {
                                    input_audio: ChatCompletionContentPartInputAudio {
                                        data: data.clone(),
                                        format: audio_format_from_media_type(media_type),
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
                    ..
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
                    ..
                } => {
                    let mut parts = Vec::with_capacity(content.len());
                    for part in content {
                        match part {
                            Content::Text { text } => {
                                parts.push(ChatCompletionContentPart::Text { text: text.clone() })
                            }
                            Content::ImageUrl { .. }
                            | Content::ImageData { .. }
                            | Content::AudioData { .. } => {
                                warn!(
                                    "OpenAI-compatible assistant responses do not support non-text content yet"
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
                Message::ToolResult { call, result, .. } => {
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
        request: GenerationRequest<'a>,
    ) -> BabataResult<GenerationResponse> {
        let request_body = ChatCompletionRequest {
            model: request.model.to_string(),
            messages: self.format_messages(
                request.system_prompts,
                request.context,
                request.prompts,
            )?,
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
                        task_id: Uuid::nil(),
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
                task_id: Uuid::nil(),
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

fn audio_format_from_media_type(media_type: &MediaType) -> String {
    if let Some(format) = media_type.audio_format() {
        return format.to_string();
    }
    media_type.as_mime_str()
}

fn build_combined_system_prompt(system_prompts: &[String], context: &str) -> Option<String> {
    let mut sections = system_prompts
        .iter()
        .filter(|system_prompt| !system_prompt.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    if !context.trim().is_empty() {
        sections.push(format!("Context:\n{context}"));
    }

    (!sections.is_empty()).then(|| sections.join("\n\n"))
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
    Text {
        text: String,
    },
    ImageUrl {
        image_url: ChatCompletionContentPartImage,
    },
    InputAudio {
        input_audio: ChatCompletionContentPartInputAudio,
    },
    File {
        file: ChatCompletionContentPartFile,
    },
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
    Function { function: FunctionDefinition },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionDefinition {
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
    use uuid::Uuid;

    use crate::{
        message::{Content, MediaType, Message},
        tool::ToolSpec,
    };

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

    #[test]
    fn format_messages_maps_audio_data_to_input_audio() {
        let provider = OpenAICompatibleProvider::new("test-key", "https://example.com/v1");
        let messages = vec![Message::UserPrompt {
            task_id: Uuid::nil(),
            content: vec![Content::AudioData {
                data: "base64-audio".to_string(),
                media_type: MediaType::AudioMp3,
            }],
        }];

        let payload = provider
            .format_messages(&[], "", &messages)
            .expect("format messages");
        let payload = serde_json::to_value(payload).expect("serialize formatted messages");

        assert_eq!(payload[0]["role"], json!("user"));
        assert_eq!(payload[0]["content"][0]["type"], json!("input_audio"));
        assert_eq!(
            payload[0]["content"][0]["input_audio"]["data"],
            json!("base64-audio")
        );
        assert_eq!(
            payload[0]["content"][0]["input_audio"]["format"],
            json!("mp3")
        );
    }

    #[test]
    fn format_messages_places_context_before_prompts() {
        let provider = OpenAICompatibleProvider::new("test-key", "https://example.com/v1");
        let prompts = vec![Message::UserPrompt {
            task_id: Uuid::nil(),
            content: vec![Content::Text {
                text: "latest prompt".to_string(),
            }],
        }];

        let payload = provider
            .format_messages(&[], "previous context", &prompts)
            .expect("format messages");
        let payload = serde_json::to_value(payload).expect("serialize formatted messages");

        assert_eq!(payload[0]["role"], json!("system"));
        assert_eq!(payload[0]["content"], json!("Context:\nprevious context"));
        assert_eq!(payload[1]["role"], json!("user"));
        assert_eq!(payload[1]["content"][0]["text"], json!("latest prompt"));
    }

    #[test]
    fn format_messages_keeps_multiple_system_prompts_before_context() {
        let provider = OpenAICompatibleProvider::new("test-key", "https://example.com/v1");
        let system_prompts = vec!["first rules".to_string(), "second rules".to_string()];
        let prompts = vec![Message::UserPrompt {
            task_id: Uuid::nil(),
            content: vec![Content::Text {
                text: "latest prompt".to_string(),
            }],
        }];

        let payload = provider
            .format_messages(&system_prompts, "previous context", &prompts)
            .expect("format messages");
        let payload = serde_json::to_value(payload).expect("serialize formatted messages");

        assert_eq!(payload[0]["role"], json!("system"));
        assert_eq!(payload[0]["content"], json!("first rules"));
        assert_eq!(payload[1]["role"], json!("system"));
        assert_eq!(payload[1]["content"], json!("second rules"));
        assert_eq!(payload[2]["role"], json!("system"));
        assert_eq!(payload[2]["content"], json!("Context:\nprevious context"));
        assert_eq!(payload[3]["role"], json!("user"));
    }

    #[test]
    fn format_messages_can_combine_system_prompts_and_context_into_one_message() {
        let provider = OpenAICompatibleProvider::new("test-key", "https://example.com/v1")
            .with_combined_system_prompt(true);
        let system_prompts = vec!["first rules".to_string(), "second rules".to_string()];
        let prompts = vec![Message::UserPrompt {
            task_id: Uuid::nil(),
            content: vec![Content::Text {
                text: "latest prompt".to_string(),
            }],
        }];

        let payload = provider
            .format_messages(&system_prompts, "previous context", &prompts)
            .expect("format messages");
        let payload = serde_json::to_value(payload).expect("serialize formatted messages");

        assert_eq!(payload[0]["role"], json!("system"));
        assert_eq!(
            payload[0]["content"],
            json!("first rules\n\nsecond rules\n\nContext:\nprevious context")
        );
        assert_eq!(payload[1]["role"], json!("user"));
    }
}
