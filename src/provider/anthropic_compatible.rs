use chrono::Utc;
use log::{debug, warn};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    BabataResult,
    error::BabataError,
    message::{Content, Message, ToolCall},
    provider::{GenerationRequest, GenerationResponse, Provider},
    tool::ToolSpec,
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
                input_schema: t.parameters.clone().to_value(),
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
                    media_type: *media_type,
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

    fn format_messages(&self, prompts: &[Message]) -> BabataResult<Vec<AnthropicMessage>> {
        let mut request_messages: Vec<AnthropicMessage> = Vec::with_capacity(prompts.len());

        for message in prompts {
            let (role, blocks) = match message {
                Message::UserPrompt { content, .. } | Message::UserSteering { content, .. } => {
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
                    (AnthropicRole::User, blocks)
                }
                Message::AssistantToolCalls {
                    calls,
                    reasoning_content,
                    ..
                } => {
                    let mut blocks = Vec::new();
                    if let Some(rc) = reasoning_content {
                        // The generic Message model does not store provider-specific signatures.
                        // Therefore we send an empty signature here. This is a design boundary:
                        // round-tripping a thinking block through the generic Message model will
                        // lose the original signature, and the provider may reject it on
                        // subsequent turns. A provider-specific storage layer would be needed
                        // to preserve signatures across turns.
                        blocks.push(AnthropicContentBlock::Thinking {
                            thinking: rc.clone(),
                            signature: String::new(),
                        });
                    }
                    blocks.extend(calls.iter().map(|call| {
                        let input: Value = serde_json::from_str(&call.args)
                            .unwrap_or_else(|_| Value::Object(Default::default()));
                        AnthropicContentBlock::ToolUse {
                            id: call.call_id.clone(),
                            name: call.tool_name.clone(),
                            input,
                        }
                    }));
                    (AnthropicRole::Assistant, blocks)
                }
                Message::AssistantResponse {
                    content,
                    reasoning_content,
                    ..
                } => {
                    let mut blocks = Vec::new();
                    if let Some(rc) = reasoning_content {
                        // The generic Message model does not store provider-specific signatures.
                        // Therefore we send an empty signature here. This is a design boundary:
                        // round-tripping a thinking block through the generic Message model will
                        // lose the original signature, and the provider may reject it on
                        // subsequent turns. A provider-specific storage layer would be needed
                        // to preserve signatures across turns.
                        blocks.push(AnthropicContentBlock::Thinking {
                            thinking: rc.clone(),
                            signature: String::new(),
                        });
                    }
                    for part in content {
                        match self.format_content_block(part) {
                            Ok(block) => blocks.push(block),
                            Err(e) => {
                                warn!("Skipping unsupported content in assistant message: {e}");
                                continue;
                            }
                        }
                    }
                    (AnthropicRole::Assistant, blocks)
                }
                Message::ToolResult { call, result, .. } => (
                    AnthropicRole::User,
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
                role,
                content: blocks,
            });
        }

        Ok(request_messages)
    }

    fn parse_response(&self, response_body: AnthropicResponse) -> BabataResult<GenerationResponse> {
        let mut tool_calls = Vec::new();
        let mut text_content = Vec::new();
        let mut thinking_parts = Vec::new();

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
                AnthropicContentBlock::Thinking { thinking, .. } => {
                    thinking_parts.push(thinking);
                }
                _ => {}
            }
        }

        let reasoning_content = if !thinking_parts.is_empty() {
            Some(thinking_parts.join("\n"))
        } else {
            None
        };

        if !tool_calls.is_empty() {
            // For tool_calls path, if no thinking block was present but there is text content,
            // fall back to using the text as reasoning_content to preserve backward-compatible
            // behavior (older models may emit plain text before tool_use without a thinking block).
            let reasoning_content = reasoning_content.or_else(|| {
                if text_content.is_empty() {
                    None
                } else {
                    let texts: Vec<String> = text_content
                        .iter()
                        .filter_map(|c| match c {
                            Content::Text { text } => Some(text.clone()),
                            _ => None,
                        })
                        .collect();
                    Some(texts.join("\n"))
                }
            });
            return Ok(GenerationResponse {
                message: Message::AssistantToolCalls {
                    calls: tool_calls,
                    reasoning_content,
                    created_at: Utc::now(),
                },
            });
        }

        if text_content.is_empty() && reasoning_content.is_none() {
            return Err(BabataError::provider("No content in assistant message"));
        }

        Ok(GenerationResponse {
            message: Message::AssistantResponse {
                content: text_content,
                reasoning_content,
                created_at: Utc::now(),
            },
        })
    }
}

#[async_trait::async_trait]
impl Provider for AnthropicCompatibleProvider {
    fn name() -> &'static str {
        "anthropic-compatible"
    }

    async fn generate<'a>(
        &self,
        request: GenerationRequest<'a>,
    ) -> BabataResult<GenerationResponse> {
        let system = build_system_blocks(request.system_prompts, request.context);

        let request_body = AnthropicRequest {
            model: request.model.to_string(),
            max_tokens: DEFAULT_MAX_TOKENS,
            system,
            messages: self.format_messages(request.prompts)?,
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

        let raw_response_body = response
            .text()
            .await
            .map_err(|e| BabataError::provider(format!("Failed to read response body: {e}")))?;
        let response_body: AnthropicResponse =
            serde_json::from_str(&raw_response_body).map_err(|e| {
                BabataError::provider(format!(
                    "Failed to parse response body: {e}. Response body: {raw_response_body}"
                ))
            })?;

        debug!(
            "Anthropic-compatible API response: {}",
            serde_json::to_string_pretty(&response_body)?
        );

        self.parse_response(response_body)
    }
}

fn build_system_blocks(
    system_prompts: &[String],
    context: &str,
) -> Option<Vec<AnthropicSystemBlock>> {
    let mut blocks = Vec::new();
    for system_prompt in system_prompts {
        if system_prompt.is_empty() {
            continue;
        }
        blocks.push(AnthropicSystemBlock::Text {
            text: system_prompt.to_string(),
        });
    }
    if !context.trim().is_empty() {
        blocks.push(AnthropicSystemBlock::Text {
            text: format!("Context:\n{context}"),
        });
    }

    (!blocks.is_empty()).then_some(blocks)
}

// Anthropic API types

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<Vec<AnthropicSystemBlock>>,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicSystemBlock {
    Text { text: String },
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum AnthropicRole {
    User,
    Assistant,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicMessage {
    role: AnthropicRole,
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
    Thinking {
        thinking: String,
        signature: String,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicImageSource {
    Base64 {
        media_type: crate::message::MediaType,
        data: String,
    },
    Url {
        url: String,
    },
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
    use crate::provider::Provider;
    use axum::{Router, routing::post};
    use serde_json::json;
    use tokio::net::TcpListener;

    #[test]
    fn build_system_blocks_keeps_system_prompts_and_context_separate() {
        let blocks = build_system_blocks(
            &["system rules".to_string(), "more rules".to_string()],
            "memory context",
        );
        assert_eq!(
            serde_json::to_value(blocks).expect("serialize system blocks"),
            json!([
                {
                    "type": "text",
                    "text": "system rules"
                },
                {
                    "type": "text",
                    "text": "more rules"
                },
                {
                    "type": "text",
                    "text": "Context:\nmemory context"
                }
            ])
        );
    }

    #[tokio::test]
    async fn test_connection_uses_generate_endpoint() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test listener");
        let addr = listener.local_addr().expect("listener addr");
        tokio::spawn(async move {
            axum::serve(
                listener,
                Router::new().route(
                    "/v1/messages",
                    post(|| async {
                        axum::Json(json!({
                            "id": "msg-test",
                            "type": "message",
                            "role": "assistant",
                            "content": [{
                                "type": "text",
                                "text": "ok"
                            }],
                            "model": "test-model",
                            "stop_reason": "end_turn"
                        }))
                    }),
                ),
            )
            .await
            .expect("serve test app");
        });

        let provider = AnthropicCompatibleProvider::new("test-key", &format!("http://{addr}"));
        provider
            .test_connection("test-model")
            .await
            .expect("test connection should succeed");
    }

    // --- Thinking block tests ---

    #[test]
    fn test_thinking_block_serde() {
        let block = AnthropicContentBlock::Thinking {
            thinking: "Let me think step by step...".to_string(),
            signature: "sig-abc123".to_string(),
        };
        let json_value = serde_json::to_value(&block).expect("serialize thinking block");
        assert_eq!(
            json_value,
            json!({
                "type": "thinking",
                "thinking": "Let me think step by step...",
                "signature": "sig-abc123"
            })
        );

        let deserialized: AnthropicContentBlock =
            serde_json::from_value(json_value).expect("deserialize thinking block");
        match deserialized {
            AnthropicContentBlock::Thinking {
                thinking,
                signature,
            } => {
                assert_eq!(thinking, "Let me think step by step...");
                assert_eq!(signature, "sig-abc123");
            }
            other => panic!("expected Thinking variant, got {other:?}"),
        }
    }

    #[test]
    fn test_format_messages_encodes_reasoning_content_as_thinking_block_for_assistant_response() {
        let provider = AnthropicCompatibleProvider::new("test-key", "http://localhost");
        let messages = vec![Message::AssistantResponse {
            content: vec![Content::Text {
                text: "The answer is 42.".to_string(),
            }],
            reasoning_content: Some("I need to calculate...".to_string()),
            created_at: chrono::Utc::now(),
        }];
        let anthropic_messages = provider
            .format_messages(&messages)
            .expect("format messages");
        assert_eq!(anthropic_messages.len(), 1);
        assert_eq!(anthropic_messages[0].role, AnthropicRole::Assistant);
        assert_eq!(anthropic_messages[0].content.len(), 2);

        match &anthropic_messages[0].content[0] {
            AnthropicContentBlock::Thinking {
                thinking,
                signature,
            } => {
                assert_eq!(thinking, "I need to calculate...");
                assert!(signature.is_empty());
            }
            other => panic!("expected Thinking block, got {other:?}"),
        }
        match &anthropic_messages[0].content[1] {
            AnthropicContentBlock::Text { text } => {
                assert_eq!(text, "The answer is 42.");
            }
            other => panic!("expected Text block, got {other:?}"),
        }
    }

    #[test]
    fn test_format_messages_encodes_reasoning_content_as_thinking_block_for_tool_calls() {
        let provider = AnthropicCompatibleProvider::new("test-key", "http://localhost");
        let messages = vec![Message::AssistantToolCalls {
            calls: vec![ToolCall {
                call_id: "call-1".to_string(),
                tool_name: "calculator".to_string(),
                args: "{\"a\":1,\"b\":2}".to_string(),
            }],
            reasoning_content: Some("I should use a tool.".to_string()),
            created_at: chrono::Utc::now(),
        }];
        let anthropic_messages = provider
            .format_messages(&messages)
            .expect("format messages");
        assert_eq!(anthropic_messages.len(), 1);
        assert_eq!(anthropic_messages[0].role, AnthropicRole::Assistant);
        assert_eq!(anthropic_messages[0].content.len(), 2);

        match &anthropic_messages[0].content[0] {
            AnthropicContentBlock::Thinking {
                thinking,
                signature,
            } => {
                assert_eq!(thinking, "I should use a tool.");
                assert!(signature.is_empty());
            }
            other => panic!("expected Thinking block, got {other:?}"),
        }
        match &anthropic_messages[0].content[1] {
            AnthropicContentBlock::ToolUse { id, name, .. } => {
                assert_eq!(id, "call-1");
                assert_eq!(name, "calculator");
            }
            other => panic!("expected ToolUse block, got {other:?}"),
        }
    }

    #[test]
    fn test_response_parsing_extracts_thinking_block_as_reasoning_content() {
        let provider = AnthropicCompatibleProvider::new("test-key", "http://localhost");
        let response_body = AnthropicResponse {
            id: "msg-1".to_string(),
            response_type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![
                AnthropicContentBlock::Thinking {
                    thinking: "Let me analyze this...".to_string(),
                    signature: "sig-xyz".to_string(),
                },
                AnthropicContentBlock::Text {
                    text: "Here is my analysis.".to_string(),
                },
            ],
            model: "claude-3-7-sonnet".to_string(),
            stop_reason: Some("end_turn".to_string()),
        };

        let response = provider
            .parse_response(response_body)
            .expect("parse response");
        match response.message {
            Message::AssistantResponse {
                content,
                reasoning_content,
                ..
            } => {
                assert_eq!(content.len(), 1);
                assert_eq!(
                    content[0],
                    Content::Text {
                        text: "Here is my analysis.".to_string()
                    }
                );
                assert_eq!(
                    reasoning_content,
                    Some("Let me analyze this...".to_string())
                );
            }
            other => panic!("expected AssistantResponse, got {other:?}"),
        }
    }

    #[test]
    fn test_response_parsing_handles_thinking_text_and_tool_use() {
        let provider = AnthropicCompatibleProvider::new("test-key", "http://localhost");
        let response_body = AnthropicResponse {
            id: "msg-1".to_string(),
            response_type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![
                AnthropicContentBlock::Thinking {
                    thinking: "I need to search.".to_string(),
                    signature: "sig-abc".to_string(),
                },
                AnthropicContentBlock::Text {
                    text: "Let me search for you.".to_string(),
                },
                AnthropicContentBlock::ToolUse {
                    id: "tool-1".to_string(),
                    name: "search".to_string(),
                    input: json!({"query": "rust"}),
                },
            ],
            model: "claude-3-7-sonnet".to_string(),
            stop_reason: Some("tool_use".to_string()),
        };

        let response = provider
            .parse_response(response_body)
            .expect("parse response");
        match response.message {
            Message::AssistantToolCalls {
                calls,
                reasoning_content,
                ..
            } => {
                assert_eq!(calls.len(), 1);
                assert_eq!(calls[0].call_id, "tool-1");
                assert_eq!(calls[0].tool_name, "search");
                // thinking text goes to reasoning_content; regular text is not used as
                // reasoning_content because a thinking block was present
                assert_eq!(reasoning_content, Some("I need to search.".to_string()));
            }
            other => panic!("expected AssistantToolCalls, got {other:?}"),
        }
    }

    // Regression test: plain text response (no thinking block) must have reasoning_content == None
    #[test]
    fn test_response_parsing_plain_text_has_no_reasoning_content() {
        let provider = AnthropicCompatibleProvider::new("test-key", "http://localhost");
        let response_body = AnthropicResponse {
            id: "msg-1".to_string(),
            response_type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![AnthropicContentBlock::Text {
                text: "Hello, world!".to_string(),
            }],
            model: "claude-3-5-sonnet".to_string(),
            stop_reason: Some("end_turn".to_string()),
        };

        let response = provider
            .parse_response(response_body)
            .expect("parse response");
        match response.message {
            Message::AssistantResponse {
                content,
                reasoning_content,
                ..
            } => {
                assert_eq!(content.len(), 1);
                assert_eq!(
                    content[0],
                    Content::Text {
                        text: "Hello, world!".to_string()
                    }
                );
                assert_eq!(reasoning_content, None);
            }
            other => panic!("expected AssistantResponse, got {other:?}"),
        }
    }

    // Tests for AnthropicImageSource with base64 and url types
    #[test]
    fn test_base64_image_source_serde() {
        use crate::message::MediaType;

        let source = AnthropicImageSource::Base64 {
            media_type: MediaType::ImageJpeg,
            data: "base64encodeddata".to_string(),
        };
        let json_value = serde_json::to_value(&source).expect("serialize base64 source");
        assert_eq!(
            json_value,
            json!({
                "type": "base64",
                "media_type": "image/jpeg",
                "data": "base64encodeddata"
            })
        );

        // Test deserialization
        let deserialized: AnthropicImageSource =
            serde_json::from_value(json_value).expect("deserialize base64 source");
        match deserialized {
            AnthropicImageSource::Base64 { media_type, data } => {
                assert_eq!(media_type, MediaType::ImageJpeg);
                assert_eq!(data, "base64encodeddata");
            }
            _ => panic!("expected Base64 variant"),
        }
    }

    #[test]
    fn test_url_image_source_serde() {
        let source = AnthropicImageSource::Url {
            url: "https://example.com/image.png".to_string(),
        };
        let json_value = serde_json::to_value(&source).expect("serialize url source");
        assert_eq!(
            json_value,
            json!({
                "type": "url",
                "url": "https://example.com/image.png"
            })
        );

        // Test deserialization
        let deserialized: AnthropicImageSource =
            serde_json::from_value(json_value).expect("deserialize url source");
        match deserialized {
            AnthropicImageSource::Url { url } => {
                assert_eq!(url, "https://example.com/image.png");
            }
            _ => panic!("expected Url variant"),
        }
    }

    #[test]
    fn test_anthropic_message_role_serde() {
        // Test User role
        let user_msg = AnthropicMessage {
            role: AnthropicRole::User,
            content: vec![AnthropicContentBlock::Text {
                text: "hello".to_string(),
            }],
        };
        let json_value = serde_json::to_value(&user_msg).expect("serialize user message");
        assert_eq!(
            json_value,
            json!({
                "role": "user",
                "content": [{"type": "text", "text": "hello"}]
            })
        );

        // Test Assistant role
        let assistant_msg = AnthropicMessage {
            role: AnthropicRole::Assistant,
            content: vec![AnthropicContentBlock::Text {
                text: "hi".to_string(),
            }],
        };
        let json_value = serde_json::to_value(&assistant_msg).expect("serialize assistant message");
        assert_eq!(
            json_value,
            json!({
                "role": "assistant",
                "content": [{"type": "text", "text": "hi"}]
            })
        );

        // Test deserialization
        let deserialized: AnthropicMessage =
            serde_json::from_value(json_value).expect("deserialize message");
        assert!(matches!(deserialized.role, AnthropicRole::Assistant));
    }

    #[test]
    fn test_image_content_block_with_base64_source() {
        use crate::message::MediaType;

        let block = AnthropicContentBlock::Image {
            source: AnthropicImageSource::Base64 {
                media_type: MediaType::ImagePng,
                data: "pngdata".to_string(),
            },
        };
        let json_value = serde_json::to_value(&block).expect("serialize image block");
        assert_eq!(
            json_value,
            json!({
                "type": "image",
                "source": {
                    "type": "base64",
                    "media_type": "image/png",
                    "data": "pngdata"
                }
            })
        );
    }

    #[test]
    fn test_image_content_block_with_url_source() {
        let block = AnthropicContentBlock::Image {
            source: AnthropicImageSource::Url {
                url: "https://example.com/photo.jpg".to_string(),
            },
        };
        let json_value = serde_json::to_value(&block).expect("serialize image block");
        assert_eq!(
            json_value,
            json!({
                "type": "image",
                "source": {
                    "type": "url",
                    "url": "https://example.com/photo.jpg"
                }
            })
        );
    }

    // Regression test: ensure message/image block output matches Anthropic API expectations
    #[test]
    fn test_request_body_serialization_with_image_and_role() {
        let request = AnthropicRequest {
            model: "claude-3-opus-20240229".to_string(),
            max_tokens: 1024,
            system: None,
            messages: vec![
                AnthropicMessage {
                    role: AnthropicRole::User,
                    content: vec![
                        AnthropicContentBlock::Text {
                            text: "Describe this image".to_string(),
                        },
                        AnthropicContentBlock::Image {
                            source: AnthropicImageSource::Url {
                                url: "https://example.com/image.jpg".to_string(),
                            },
                        },
                    ],
                },
                AnthropicMessage {
                    role: AnthropicRole::Assistant,
                    content: vec![AnthropicContentBlock::Text {
                        text: "I see...".to_string(),
                    }],
                },
            ],
            tools: None,
        };

        let json_value = serde_json::to_value(&request).expect("serialize request");
        let expected = json!({
            "model": "claude-3-opus-20240229",
            "max_tokens": 1024,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {"type": "text", "text": "Describe this image"},
                        {
                            "type": "image",
                            "source": {
                                "type": "url",
                                "url": "https://example.com/image.jpg"
                            }
                        }
                    ]
                },
                {
                    "role": "assistant",
                    "content": [
                        {"type": "text", "text": "I see..."}
                    ]
                }
            ]
        });

        assert_eq!(json_value, expected);
    }

    // Regression test: ensure Content::ImageUrl is correctly mapped to AnthropicImageSource::Url
    #[test]
    fn test_format_content_block_maps_image_url_to_anthropic_url_source() {
        let provider = AnthropicCompatibleProvider::new("test-key", "http://localhost");
        let content = Content::ImageUrl {
            url: "https://example.com/image.jpg".to_string(),
        };
        let block = provider
            .format_content_block(&content)
            .expect("should map image URL");
        match block {
            AnthropicContentBlock::Image {
                source: AnthropicImageSource::Url { url },
            } => {
                assert_eq!(url, "https://example.com/image.jpg");
            }
            other => panic!("expected Image block with Url source, got {other:?}"),
        }
    }

    #[test]
    fn test_format_content_block_maps_image_data_to_anthropic_base64_source() {
        use crate::message::MediaType;

        let provider = AnthropicCompatibleProvider::new("test-key", "http://localhost");
        let content = Content::ImageData {
            data: "base64data".to_string(),
            media_type: MediaType::ImageWebp,
        };
        let block = provider
            .format_content_block(&content)
            .expect("should map image data");
        match block {
            AnthropicContentBlock::Image {
                source: AnthropicImageSource::Base64 { media_type, data },
            } => {
                assert_eq!(media_type, MediaType::ImageWebp);
                assert_eq!(data, "base64data");
            }
            other => panic!("expected Image block with Base64 source, got {other:?}"),
        }
    }

    // Regression test: ensure a full UserPrompt with ImageUrl serializes correctly
    #[test]
    fn test_format_messages_with_image_url_content() {
        use crate::message::MediaType;

        let provider = AnthropicCompatibleProvider::new("test-key", "http://localhost");
        let messages = vec![Message::UserPrompt {
            content: vec![
                Content::Text {
                    text: "What's in this image?".to_string(),
                },
                Content::ImageUrl {
                    url: "https://cdn.example.com/photo.png".to_string(),
                },
                Content::ImageData {
                    data: "abc123".to_string(),
                    media_type: MediaType::ImagePng,
                },
            ],
            created_at: chrono::Utc::now(),
        }];
        let anthropic_messages = provider
            .format_messages(&messages)
            .expect("format messages");
        assert_eq!(anthropic_messages.len(), 1);
        assert_eq!(anthropic_messages[0].role, AnthropicRole::User);
        assert_eq!(anthropic_messages[0].content.len(), 3);

        // Verify serialization produces correct JSON
        let json_value = serde_json::to_value(&anthropic_messages[0]).expect("serialize message");
        let expected = json!({
            "role": "user",
            "content": [
                {"type": "text", "text": "What's in this image?"},
                {
                    "type": "image",
                    "source": {
                        "type": "url",
                        "url": "https://cdn.example.com/photo.png"
                    }
                },
                {
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/png",
                        "data": "abc123"
                    }
                }
            ]
        });
        assert_eq!(json_value, expected);
    }
}
