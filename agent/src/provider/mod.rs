mod openai;

pub use openai::*;

use std::fmt::Debug;

use crate::{BabataResult, message::Message, tool::ToolSpec};

#[async_trait::async_trait]
pub trait Provider: Debug + Send + Sync {
    // Name of the provider, e.g., "OpenAI", "Anthropic", "Google Gemini"
    fn name(&self) -> &str;
    // Model name, e.g., "gpt-4", "gpt-3.5-turbo", "claude-2", "gemini-1.5-pro"
    fn model(&self) -> &str;
    async fn generate<'a>(&self, request: GenerationReqest<'a>)
    -> BabataResult<GenerationResponse>;
    async fn interact(&self, request: InteractionRequest) -> BabataResult<InteractionResponse>;
}

pub struct GenerationReqest<'a> {
    pub system_prompt: &'a str,
    pub messages: &'a [Message],
    pub tools: &'a [ToolSpec],
}

pub struct GenerationResponse {
    pub message: Message,
}

pub struct InteractionRequest {}

pub struct InteractionResponse {}
