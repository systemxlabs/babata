mod store;

pub use store::*;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    // User question / instruction
    User,
    // Assistant answer / thinking / tool call instruction
    Assistant,
    // Tool call result
    Tool,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::Tool => write!(f, "tool"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    UserPrompt { content: Vec<Content> },
    AssistantResponse { content: Vec<Content> },
    AssistantToolCalls { calls: Vec<ToolCall> },
    ToolResult { call: ToolCall, result: String },
}

impl Message {
    pub fn role(&self) -> Role {
        match self {
            Message::UserPrompt { .. } => Role::User,
            Message::AssistantResponse { .. } => Role::Assistant,
            Message::AssistantToolCalls { .. } => Role::Assistant,
            Message::ToolResult { .. } => Role::Tool,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub call_id: String,
    pub tool_name: String,
    pub args: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Content {
    Text { text: String },
    ImageUrl { url: String },
    ImageData { data: String, media_type: String },
}
