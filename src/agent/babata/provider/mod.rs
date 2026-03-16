mod anthropic;
mod anthropic_compatible;
mod custom;
mod deepseek;
mod kimi;
mod moonshot;
mod openai;
mod openai_compatible;

pub use anthropic::*;
pub(crate) use anthropic_compatible::*;
pub use custom::*;
pub use deepseek::*;
pub use kimi::*;
pub use moonshot::*;
pub use openai::*;
pub(crate) use openai_compatible::*;

use std::{collections::HashMap, fmt::Debug, sync::Arc};

use crate::{
    BabataResult,
    agent::babata::ToolSpec,
    config::{Config, ProviderConfig},
    message::Message,
};

#[async_trait::async_trait]
pub trait Provider: Debug + Send + Sync {
    // Name of the provider, e.g., "OpenAI", "Anthropic", "Google Gemini"
    fn name() -> &'static str
    where
        Self: Sized;

    fn supported_models() -> &'static [Model]
    where
        Self: Sized;

    async fn generate<'a>(
        &self,
        request: GenerationRequest<'a>,
    ) -> BabataResult<GenerationResponse>;
    async fn interact(&self, request: InteractionRequest) -> BabataResult<InteractionResponse>;
}

pub struct GenerationRequest<'a> {
    pub system_prompt: &'a str,
    pub model: &'a str,
    pub prompts: &'a [Message],
    pub context: &'a str,
    pub tools: &'a [ToolSpec],
}

pub struct GenerationResponse {
    pub message: Message,
}

pub struct InteractionRequest {}

pub struct InteractionResponse {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Model {
    pub provider: &'static str,
    pub name: &'static str,
    pub context_length: usize,
}

pub fn create_provider(provider_config: &ProviderConfig) -> BabataResult<Arc<dyn Provider>> {
    match provider_config {
        ProviderConfig::OpenAI(config) => Ok(Arc::new(OpenAIProvider::new(&config.api_key))),
        ProviderConfig::Kimi(config) => Ok(Arc::new(KimiProvider::new(&config.api_key))),
        ProviderConfig::Moonshot(config) => Ok(Arc::new(MoonshotProvider::new(&config.api_key))),
        ProviderConfig::DeepSeek(config) => Ok(Arc::new(DeepSeekProvider::new(&config.api_key))),
        ProviderConfig::Anthropic(config) => Ok(Arc::new(AnthropicProvider::new(&config.api_key))),
        ProviderConfig::Custom(config) => Ok(Arc::new(CustomProvider::new(config))),
    }
}

pub fn build_providers(config: &Config) -> BabataResult<HashMap<String, Arc<dyn Provider>>> {
    let mut providers: HashMap<String, Arc<dyn Provider>> =
        HashMap::with_capacity(config.providers.len());

    for provider_config in &config.providers {
        let provider_name = provider_config.name();
        let provider = create_provider(provider_config)?;
        providers.insert(provider_name.to_string(), provider);
    }

    Ok(providers)
}
