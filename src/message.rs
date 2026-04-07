use serde::{Deserialize, Serialize, de::value::StringDeserializer};

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
    UserPrompt {
        content: Vec<Content>,
    },
    UserSteering {
        content: Vec<Content>,
    },
    AssistantResponse {
        content: Vec<Content>,
        reasoning_content: Option<String>,
    },
    AssistantToolCalls {
        calls: Vec<ToolCall>,
        reasoning_content: Option<String>,
    },
    ToolResult {
        call: ToolCall,
        result: String,
    },
}

impl Message {
    pub fn role(&self) -> Role {
        match self {
            Message::UserPrompt { .. } => Role::User,
            Message::UserSteering { .. } => Role::User,
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
    pub args: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Content {
    Text { text: String },
    ImageUrl { url: String },
    ImageData { data: String, media_type: MediaType },
    AudioData { data: String, media_type: MediaType },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaType {
    #[serde(rename = "image/png")]
    ImagePng,
    #[serde(rename = "image/jpeg")]
    ImageJpeg,
    #[serde(rename = "image/webp")]
    ImageWebp,
    #[serde(rename = "image/gif")]
    ImageGif,
    #[serde(rename = "audio/mp3")]
    AudioMp3,
    #[serde(rename = "audio/mpeg")]
    AudioMpeg,
    #[serde(rename = "audio/wav")]
    AudioWav,
    #[serde(rename = "audio/ogg")]
    AudioOgg,
    #[serde(rename = "audio/webm")]
    AudioWebm,
}

impl MediaType {
    pub fn from_mime(mime: &str) -> Option<Self> {
        let normalized = mime.to_ascii_lowercase();
        let deserializer = StringDeserializer::<serde::de::value::Error>::new(normalized);
        Self::deserialize(deserializer).ok()
    }

    pub fn as_mime_str(&self) -> String {
        serde_json::to_value(self)
            .expect("MediaType should serialize to JSON value")
            .as_str()
            .expect("MediaType should serialize as JSON string")
            .to_string()
    }

    pub fn audio_format(&self) -> Option<&'static str> {
        match self {
            Self::AudioMp3 => Some("mp3"),
            Self::AudioMpeg => Some("mpeg"),
            Self::AudioWav => Some("wav"),
            Self::AudioOgg => Some("ogg"),
            Self::AudioWebm => Some("webm"),
            Self::ImagePng | Self::ImageJpeg | Self::ImageWebp | Self::ImageGif => None,
        }
    }
}

impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_mime_str())
    }
}

#[cfg(test)]
mod tests {
    use super::MediaType;

    #[test]
    fn media_type_serializes_as_mime_string() {
        let serialized = serde_json::to_string(&MediaType::ImagePng).expect("serialize media type");
        assert_eq!(serialized, "\"image/png\"");
    }

    #[test]
    fn media_type_deserializes_unknown_as_error() {
        let parsed = serde_json::from_str::<MediaType>("\"audio/flac\"");
        assert!(parsed.is_err());
    }
}
