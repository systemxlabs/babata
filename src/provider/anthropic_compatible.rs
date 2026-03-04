use log::{debug, warn};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    BabataResult,
    error::BabataError,
    message::{Content, Message, ToolCall},
    provider::{GenerationReqest, GenerationResponse, InteractionRequest, InteractionResponse},
    tool::ToolSpec,
};

const ANTHROPIC_API_VERSION: &str = "2023-06-01";
const DEFAULT_MAX_TOKENS: u32 = 4096;

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
            Content::ImageUrl { url } => Ok(AnthropicContentBlock::Image {
                source: AnthropicImageSource::Url { url: url.clone() },
            }),
            Content::ImageData { data, media_type } => Ok(AnthropicContentBlock::Image {
                source: AnthropicImageSource::Base64 {
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

    fn format_messages(&self, messages: &[Message]) -> BabataResult<Vec<AnthropicMessage>> {
        let mut request_messages = Vec::with_capacity(messages.len());

        for message in messages {
            match message {
                Message::UserPrompt { content } => {
                    let mut blocks = Vec::new();
                    for part in content {
                        match self.format_content_block(part) {
                            Ok(block) => blocks.push(block),
                            Err(_) => continue, // Skip unsupported content types
                        }
                    }

                    if !blocks.is_empty() {
                        request_messages.push(AnthropicMessage {
                            role: "user".to_string(),
                            content: blocks,
                        });
                    }
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

                    request_messages.push(AnthropicMessage {
                        role: "assistant".to_string(),
                        content: blocks,
                    });
                }
                Message::AssistantResponse {
                    content,
                    reasoning_content: _,
                } => {
                    let mut blocks = Vec::new();
                    for part in content {
                        match self.format_content_block(part) {
                            Ok(block) => blocks.push(block),
                            Err(_) => continue,
                        }
                    }

                    if !blocks.is_empty() {
                        request_messages.push(AnthropicMessage {
                            role: "assistant".to_string(),
                            content: blocks,
                        });
                    }
                }
                Message::ToolResult { call, result } => {
                    request_messages.push(AnthropicMessage {
                        role: "user".to_string(),
                        content: vec![AnthropicContentBlock::ToolResult {
                            tool_use_id: call.call_id.clone(),
                            content: result.clone(),
                        }],
                    });
                }
            }
        }

        Ok(request_messages)
    }

    pub async fn generate<'a>(
        &self,
        request: GenerationReqest<'a>,
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
            messages: self.format_messages(request.messages)?,
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
            return Ok(GenerationResponse {
                message: Message::AssistantToolCalls {
                    calls: tool_calls,
                    reasoning_content: None,
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
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicImageSource {
    Url { url: String },
    Base64 { media_type: String, data: String },
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
        message::{Content, Message},
        tool::ToolSpec,
    };
    use serde_json::json;

    #[tokio::test]
    #[ignore]
    async fn integration_test_generate_simple_text() {
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

        let request = crate::provider::GenerationReqest {
            system_prompt: "",
            model: "claude-opus-4-6",
            messages: &messages,
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
    async fn integration_test_generate_with_tools() {
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

        let request = crate::provider::GenerationReqest {
            model: "claude-opus-4-6",
            system_prompt: "",
            messages: &messages,
            tools: &tools,
        };

        let response = provider.generate(request).await.expect("generate failed");

        // Extract tool calls
        let tool_calls = match response.message {
            Message::AssistantToolCalls {
                calls,
                reasoning_content,
            } => {
                assert!(!calls.is_empty());
                println!("\n=== Tool Calls ===");
                for (i, call) in calls.iter().enumerate() {
                    println!("Call #{}: {}", i + 1, call.tool_name);
                    println!("  ID: {}", call.call_id);
                    println!("  Arguments: {}", call.args);
                }
                if let Some(reasoning) = &reasoning_content {
                    println!("\n=== Reasoning Content ===");
                    println!("{}", reasoning);
                }
                assert_eq!(calls[0].tool_name, "get_weather");

                // Add assistant's tool calls to message history
                messages.push(Message::AssistantToolCalls {
                    calls: calls.clone(),
                    reasoning_content,
                });
                calls
            }
            _ => panic!("Expected AssistantToolCalls"),
        };

        // Simulate tool execution and add results
        for call in tool_calls {
            let mock_result = json!({
                "location": "San Francisco",
                "temperature": 18,
                "condition": "Sunny",
                "humidity": 65
            })
            .to_string();

            println!("\n=== Tool Result ===");
            println!("Tool: {}", call.tool_name);
            println!("Result: {}", mock_result);

            messages.push(Message::ToolResult {
                call,
                result: mock_result,
            });
        }

        // Second request: get final response with tool results
        let request = crate::provider::GenerationReqest {
            model: "claude-opus-4-6",
            system_prompt: "",
            messages: &messages,
            tools: &tools,
        };

        let response = provider.generate(request).await.expect("generate failed");

        // Get final text response
        match response.message {
            Message::AssistantResponse {
                content,
                reasoning_content,
            } => {
                println!("\n=== Final Response ===");
                for part in &content {
                    if let Content::Text { text } = part {
                        println!("{}", text);
                    }
                }
                if let Some(reasoning) = reasoning_content {
                    println!("\n=== Final Reasoning ===");
                    println!("{}", reasoning);
                }
                assert!(!content.is_empty());
            }
            _ => panic!("Expected AssistantResponse with final answer"),
        }
    }
}
