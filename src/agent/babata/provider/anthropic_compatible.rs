use log::{debug, warn};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    BabataResult,
    agent::babata::ToolSpec,
    agent::babata::{
        GenerationRequest, GenerationResponse, InteractionRequest, InteractionResponse,
    },
    error::BabataError,
    message::{Content, Message, ToolCall},
};

const ANTHROPIC_API_VERSION: &str = "2023-06-01";
const DEFAULT_MAX_TOKENS: u32 = 8192;

#[derive(Debug)]
pub struct AnthropicCompatibleProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl AnthropicCompatibleProvider {
    pub fn new(api_key: &str, base_url: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
        }
    }

    fn format_tools(&self, tools: &[ToolSpec]) -> Vec<AnthropicTool> {
        tools
            .iter()
            .map(|t| AnthropicTool {
                name: t.name.clone(),
                description: t.description.clone(),
                input_schema: t.parameters.clone(),
            })
            .collect()
    }

    fn format_content_block(&self, content: &Content) -> BabataResult<AnthropicContentBlock> {
        match content {
            Content::Text { text } => Ok(AnthropicContentBlock::Text { text: text.clone() }),
            Content::ImageUrl { .. } => {
                warn!(
                    "Anthropic-compatible API does not support image URL source, only base64 - skipping image content"
                );
                Err(BabataError::provider(
                    "Anthropic-compatible API does not support image URL source, only base64",
                ))
            }
            Content::ImageData { data, media_type } => Ok(AnthropicContentBlock::Image {
                source: AnthropicImageSource {
                    source_type: "base64".to_string(),
                    media_type: media_type.as_mime_str(),
                    data: data.clone(),
                },
            }),
            Content::AudioData { .. } => {
                warn!(
                    "Anthropic-compatible API does not support audio input - skipping audio content"
                );
                Err(BabataError::provider(
                    "Anthropic-compatible API does not support audio input",
                ))
            }
        }
    }

    fn format_messages(
        &self,
        context: &[Message],
        prompts: &[Message],
    ) -> BabataResult<Vec<AnthropicMessage>> {
        let mut request_messages: Vec<AnthropicMessage> =
            Vec::with_capacity(context.len() + prompts.len());

        for message in context.iter().chain(prompts.iter()) {
            let (role, blocks) = match message {
                Message::UserPrompt { content } => {
                    let mut blocks = Vec::new();
                    for part in content {
                        match self.format_content_block(part) {
                            Ok(block) => blocks.push(block),
                            Err(e) => {
                                warn!("Skipping unsupported content in user message: {e}");
                                continue;
                            }
                        }
                    }
                    ("user", blocks)
                }
                Message::AssistantToolCalls {
                    calls,
                    reasoning_content: _,
                } => {
                    let blocks = calls
                        .iter()
                        .map(|call| {
                            let input: Value = serde_json::from_str(&call.args)
                                .unwrap_or_else(|_| Value::Object(Default::default()));
                            AnthropicContentBlock::ToolUse {
                                id: call.call_id.clone(),
                                name: call.tool_name.clone(),
                                input,
                            }
                        })
                        .collect();
                    ("assistant", blocks)
                }
                Message::AssistantResponse {
                    content,
                    reasoning_content: _,
                } => {
                    let mut blocks = Vec::new();
                    for part in content {
                        match self.format_content_block(part) {
                            Ok(block) => blocks.push(block),
                            Err(e) => {
                                warn!("Skipping unsupported content in assistant message: {e}");
                                continue;
                            }
                        }
                    }
                    ("assistant", blocks)
                }
                Message::ToolResult { call, result } => (
                    "user",
                    vec![AnthropicContentBlock::ToolResult {
                        tool_use_id: call.call_id.clone(),
                        content: result.clone(),
                    }],
                ),
            };

            if blocks.is_empty() {
                continue;
            }

            // Merge consecutive messages with the same role
            if let Some(last) = request_messages.last_mut()
                && last.role == role
            {
                last.content.extend(blocks);
                continue;
            }

            request_messages.push(AnthropicMessage {
                role: role.to_string(),
                content: blocks,
            });
        }

        Ok(request_messages)
    }

    pub async fn generate<'a>(
        &self,
        request: GenerationRequest<'a>,
    ) -> BabataResult<GenerationResponse> {
        let system_prompt = request.system_prompt.trim();
        let system = if system_prompt.is_empty() {
            None
        } else {
            Some(system_prompt.to_string())
        };

        let request_body = AnthropicRequest {
            model: request.model.to_string(),
            max_tokens: DEFAULT_MAX_TOKENS,
            system,
            messages: self.format_messages(request.context, request.prompts)?,
            tools: (!request.tools.is_empty()).then(|| self.format_tools(request.tools)),
        };

        debug!(
            "Sending Anthropic-compatible API request to {}: {}",
            self.base_url,
            serde_json::to_string_pretty(&request_body)?
        );

        let response = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_API_VERSION)
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                BabataError::provider(format!(
                    "Failed to send request to Anthropic-compatible API ({}): {}",
                    self.base_url, e
                ))
            })?;

        let status = response.status();
        if status != StatusCode::OK {
            let body = response.text().await.unwrap_or_default();
            return Err(BabataError::provider(format!(
                "Anthropic-compatible API ({}) returned error status {status}: {body}",
                self.base_url
            )));
        }

        let response_body: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| BabataError::provider(format!("Failed to parse response body: {e}")))?;

        debug!(
            "Anthropic-compatible API response: {}",
            serde_json::to_string_pretty(&response_body)?
        );

        // Check for tool use in response
        let mut tool_calls = Vec::new();
        let mut text_content = Vec::new();

        for block in response_body.content {
            match block {
                AnthropicContentBlock::Text { text } => {
                    text_content.push(Content::Text { text });
                }
                AnthropicContentBlock::ToolUse { id, name, input } => {
                    let args = serde_json::to_string(&input)?;
                    tool_calls.push(ToolCall {
                        call_id: id,
                        tool_name: name,
                        args,
                    });
                }
                _ => {}
            }
        }

        if !tool_calls.is_empty() {
            let reasoning_content = if text_content.is_empty() {
                None
            } else {
                let texts: Vec<String> = text_content
                    .into_iter()
                    .filter_map(|c| match c {
                        Content::Text { text } => Some(text),
                        _ => None,
                    })
                    .collect();
                Some(texts.join("\n"))
            };
            return Ok(GenerationResponse {
                message: Message::AssistantToolCalls {
                    calls: tool_calls,
                    reasoning_content,
                },
            });
        }

        if text_content.is_empty() {
            return Err(BabataError::provider("No content in assistant message"));
        }

        Ok(GenerationResponse {
            message: Message::AssistantResponse {
                content: text_content,
                reasoning_content: None,
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

// Anthropic API types

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContentBlock>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicContentBlock {
    Text {
        text: String,
    },
    Image {
        source: AnthropicImageSource,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicImageSource {
    #[serde(rename = "type")]
    source_type: String,
    media_type: String,
    data: String,
}

#[derive(Debug, Serialize)]
struct AnthropicTool {
    name: String,
    description: String,
    input_schema: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicResponse {
    id: String,
    #[serde(rename = "type")]
    response_type: String,
    role: String,
    content: Vec<AnthropicContentBlock>,
    model: String,
    stop_reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        agent::babata::ToolSpec,
        message::{Content, Message},
    };
    use serde_json::json;

    #[tokio::test]
    #[ignore]
    async fn test_generate_simple_text() {
        let base_url = std::env::var("ANTHROPIC_BASE_URL")
            .expect("ANTHROPIC_BASE_URL environment variable not set");
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .expect("ANTHROPIC_API_KEY environment variable not set");

        let provider = AnthropicCompatibleProvider::new(&api_key, &base_url);

        let messages = vec![Message::UserPrompt {
            content: vec![Content::Text {
                text: "Hi".to_string(),
            }],
        }];

        let request = crate::agent::babata::GenerationRequest {
            system_prompt: "",
            model: "claude-opus-4-6",
            prompts: &messages,
            context: &[],
            tools: &[],
        };

        let response = provider.generate(request).await.expect("generate failed");

        match response.message {
            Message::AssistantResponse { content, .. } => {
                assert!(!content.is_empty());
                match &content[0] {
                    Content::Text { text } => {
                        assert!(!text.is_empty());
                        println!("Assistant: {}", text);
                    }
                    other => panic!("Expected Content::Text, got {:?}", other),
                }
            }
            _ => panic!("Expected AssistantResponse"),
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_generate_with_tools() {
        let base_url = std::env::var("ANTHROPIC_BASE_URL")
            .expect("ANTHROPIC_BASE_URL environment variable not set");
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .expect("ANTHROPIC_API_KEY environment variable not set");

        let provider = AnthropicCompatibleProvider::new(&api_key, &base_url);

        let tools = vec![ToolSpec {
            name: "get_weather".to_string(),
            description: "Get the weather for a location".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "The city name"
                    }
                },
                "required": ["location"]
            }),
        }];

        // First request: user asks about weather
        let mut messages = vec![Message::UserPrompt {
            content: vec![Content::Text {
                text: "What's the weather in San Francisco?".to_string(),
            }],
        }];

        let request = crate::agent::babata::provider::GenerationRequest {
            model: "claude-opus-4-6",
            system_prompt: "",
            prompts: &messages,
            context: &[],
            tools: &tools,
        };

        let response = provider.generate(request).await.expect("generate failed");

        // Extract tool call
        let tool_call = match response.message {
            Message::AssistantToolCalls {
                calls,
                reasoning_content,
            } => {
                assert!(!calls.is_empty());
                assert_eq!(calls[0].tool_name, "get_weather");

                // Add assistant's tool calls to message history
                messages.push(Message::AssistantToolCalls {
                    calls: calls.clone(),
                    reasoning_content,
                });
                calls[0].clone()
            }
            _ => panic!("Expected AssistantToolCalls"),
        };

        // Simulate tool execution and add result
        let mock_result = json!({
            "location": "San Francisco",
            "temperature": 18,
            "condition": "Sunny",
            "humidity": 65
        })
        .to_string();
        messages.push(Message::ToolResult {
            call: tool_call,
            result: mock_result,
        });

        // Second request: get final response with tool results
        let request = crate::agent::babata::GenerationRequest {
            model: "claude-opus-4-6",
            system_prompt: "",
            prompts: &messages,
            context: &[],
            tools: &tools,
        };

        let response = provider.generate(request).await.expect("generate failed");

        // Get final text response
        match response.message {
            Message::AssistantResponse { content, .. } => {
                assert!(!content.is_empty());
                match &content[0] {
                    Content::Text { text } => {
                        assert!(!text.is_empty());
                        println!("Assistant: {}", text);
                    }
                    other => panic!("Expected Content::Text, got {:?}", other),
                }
            }
            _ => panic!("Expected AssistantResponse with final answer"),
        }
    }
}
