mod moonshot;
mod openai;

pub use moonshot::*;
pub use openai::*;

use std::{fmt::Debug, sync::Arc};

use crate::{BabataResult, config::Config, message::Message, tool::ToolSpec};

#[async_trait::async_trait]
pub trait Provider: Debug + Send + Sync {
    // Name of the provider, e.g., "OpenAI", "Anthropic", "Google Gemini"
    fn name() -> &'static str
    where
        Self: Sized;

    async fn generate<'a>(&self, request: GenerationReqest<'a>)
    -> BabataResult<GenerationResponse>;
    async fn interact(&self, request: InteractionRequest) -> BabataResult<InteractionResponse>;
}

pub struct GenerationReqest<'a> {
    pub system_prompt: &'a str,
    pub model: &'a str,
    pub messages: &'a [Message],
    pub tools: &'a [ToolSpec],
}

pub struct GenerationResponse {
    pub message: Message,
}

pub struct InteractionRequest {}

pub struct InteractionResponse {}

pub fn build_providers(config: &Config) -> Vec<Arc<dyn Provider>> {
    let mut providers: Vec<Arc<dyn Provider>> = Vec::new();

    for (provider_name, provider_config) in &config.providers {
        let provider: Arc<dyn Provider> = match provider_name.as_str() {
            "openai" => Arc::new(OpenAIProvider::new(&provider_config.api_key)),
            "moonshot" => Arc::new(MoonshotProvider::new(&provider_config.api_key)),
            name => panic!("Unknown provider '{}'", name),
        };

        providers.push(provider);
    }

    providers
}
