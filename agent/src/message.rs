use std::collections::HashMap;

pub struct Message {
    pub role: Role,
    pub kind: MessageKind,
}

pub enum Role {
    // User question / instruction
    User,
    // Assistant answer / thinking / tool call instruction
    Assistant,
    // Tool call result
    Tool,
}

pub enum MessageKind {
    UserPromptText(String),
    AssistantThoughts(String),
    ToolCall {
        tool_name: String,
        args: HashMap<String, String>,
    },
    ToolResult {
        tool_name: String,
        result: String,
    },
}

pub enum Content {}
