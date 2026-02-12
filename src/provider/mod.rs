mod moonshot;
mod openai;

pub use moonshot::*;
pub use openai::*;

use std::{collections::HashMap, fmt::Debug, sync::Arc};

use crate::{
    BabataResult,
    config::{Config, ProviderConfig},
    error::BabataError,
    message::Message,
    tool::ToolSpec,
};

#[async_trait::async_trait]
pub trait Provider: Debug + Send + Sync {
    // Name of the provider, e.g., "OpenAI", "Anthropic", "Google Gemini"
    fn name() -> &'static str
    where
        Self: Sized;

    fn supported_models() -> &'static [&'static str]
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

pub fn create_provider(
    provider_name: &str,
    provider_config: &ProviderConfig,
) -> BabataResult<Arc<dyn Provider>> {
    match provider_name.to_ascii_lowercase().as_str() {
        "openai" => Ok(Arc::new(OpenAIProvider::new(&provider_config.api_key))),
        "moonshot" => Ok(Arc::new(MoonshotProvider::new(&provider_config.api_key))),
        _ => Err(BabataError::config(format!(
            "Unsupported provider '{}'",
            provider_name
        ))),
    }
}

pub fn build_providers(config: &Config) -> BabataResult<HashMap<String, Arc<dyn Provider>>> {
    let mut providers: HashMap<String, Arc<dyn Provider>> =
        HashMap::with_capacity(config.providers.len());

    for (provider_name, provider_config) in &config.providers {
        let provider = create_provider(provider_name, provider_config)?;
        providers.insert(provider_name.clone(), provider);
    }

    Ok(providers)
}
