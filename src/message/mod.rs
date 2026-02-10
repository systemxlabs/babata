mod store;

pub use store::*;

use mime::Mime;
use serde_json::Value;

pub enum Role {
    // User question / instruction
    User,
    // Assistant answer / thinking / tool call instruction
    Assistant,
    // Tool call result
    Tool,
}

pub enum Message {
    UserPrompt(Vec<Content>),
    AssistantResponse(Vec<Content>),
    AssistantToolCalls(Vec<ToolCall>),
    ToolResult { call: ToolCall, result: String },
}

impl Message {
    pub fn role(&self) -> Role {
        match self {
            Message::UserPrompt(_) => Role::User,
            Message::AssistantResponse(_) => Role::Assistant,
            Message::AssistantToolCalls(_) => Role::Assistant,
            Message::ToolResult { .. } => Role::Tool,
        }
    }
}

pub struct ToolCall {
    pub call_id: String,
    pub tool_name: String,
    pub args: Value,
}

pub enum Content {
    Text(String),
    ImageUrl(String),
    ImageData { data: String, media_type: Mime },
}