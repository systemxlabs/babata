mod anthropic_compatible;
mod openai_compatible;

pub(crate) use anthropic_compatible::*;
pub(crate) use openai_compatible::*;

use std::{collections::HashMap, fmt::Debug, sync::Arc};

use uuid::Uuid;

use crate::{BabataResult, config::ProviderConfig, message::Message, tool::ToolSpec};

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
    pub task_id: Uuid,
    pub system_prompts: &'a [String],
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
    match provider_config.compatible_api {
        crate::config::CompatibleApi::Openai => Ok(Arc::new(OpenAICompatibleProvider::new(
            &provider_config.api_key,
            &provider_config.base_url,
        ))),
        crate::config::CompatibleApi::Anthropic => Ok(Arc::new(AnthropicCompatibleProvider::new(
            &provider_config.api_key,
            &provider_config.base_url,
        ))),
    }
}

pub fn build_providers(
    provider_configs: &[ProviderConfig],
) -> BabataResult<HashMap<String, Arc<dyn Provider>>> {
    let mut providers: HashMap<String, Arc<dyn Provider>> =
        HashMap::with_capacity(provider_configs.len());

    for provider_config in provider_configs {
        let provider_name = provider_config.name.clone();
        let provider = create_provider(provider_config)?;
        providers.insert(provider_name, provider);
    }

    Ok(providers)
}
