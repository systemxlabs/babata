use std::fmt::Debug;

use crate::{BabataResult, message::Message, tool::ToolSpec};

pub trait Provider: Debug + Send + Sync {
    // Name of the provider, e.g., "OpenAI", "Anthropic", "Google Gemini"
    fn name(&self) -> &str;
    // Model name, e.g., "gpt-4", "gpt-3.5-turbo", "claude-2", "gemini-1.5-pro"
    fn model(&self) -> &str;
    fn generate(&self, request: GenerationReqest) -> BabataResult<GenerationResponse>;
    fn interact(&self, request: IntegrationRequest) -> BabataResult<InteractionResponse>;
}

pub struct GenerationReqest<'a> {
    pub system_prompt: &'a str,
    pub messages: &'a [Message],
    pub tools: &'a [ToolSpec],
}

pub struct GenerationResponse {}

pub struct IntegrationRequest {}

pub struct InteractionResponse {}
