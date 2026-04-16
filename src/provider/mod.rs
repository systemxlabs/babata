mod anthropic_compatible;
mod config;
mod openai_compatible;

pub(crate) use anthropic_compatible::*;
pub(crate) use config::*;
pub(crate) use openai_compatible::*;

use std::{collections::HashMap, fmt::Debug, sync::Arc};

use chrono::Utc;
use uuid::Uuid;

use crate::{
    BabataResult,
    message::{Content, Message},
    tool::ToolSpec,
};

#[async_trait::async_trait]
pub trait Provider: Debug + Send + Sync {
    // Name of the provider, e.g., "OpenAI", "Anthropic", "Google Gemini"
    fn name() -> &'static str
    where
        Self: Sized;

    async fn generate<'a>(
        &self,
        request: GenerationRequest<'a>,
    ) -> BabataResult<GenerationResponse>;

    async fn test_connection(&self, model: &str) -> BabataResult<ProviderConnectionTestResult> {
        let system_prompts =
            vec!["You are a provider connection test. Reply with exactly 'ok'.".to_string()];
        let prompts = vec![Message::UserPrompt {
            content: vec![Content::Text {
                text: "Reply with exactly ok.".to_string(),
            }],
            created_at: Utc::now(),
        }];
        let tools: [ToolSpec; 0] = [];
        let started_at = std::time::Instant::now();

        let _response = self
            .generate(GenerationRequest {
                task_id: Uuid::nil(),
                system_prompts: &system_prompts,
                model,
                prompts: &prompts,
                context: "",
                tools: &tools,
            })
            .await?;

        Ok(ProviderConnectionTestResult {
            latency_ms: started_at
                .elapsed()
                .as_millis()
                .try_into()
                .unwrap_or(u64::MAX),
        })
    }
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

#[derive(Debug, Clone, Copy)]
pub struct ProviderConnectionTestResult {
    pub latency_ms: u64,
}

pub fn create_provider(provider_config: &ProviderConfig) -> BabataResult<Arc<dyn Provider>> {
    match provider_config.compatible_api {
        CompatibleApi::Openai => Ok(Arc::new(OpenAICompatibleProvider::new(
            &provider_config.api_key,
            &provider_config.base_url,
        ))),
        CompatibleApi::Anthropic => Ok(Arc::new(AnthropicCompatibleProvider::new(
            &provider_config.api_key,
            &provider_config.base_url,
        ))),
    }
}

pub async fn test_provider_connection(
    provider_config: &ProviderConfig,
    model: &str,
) -> BabataResult<ProviderConnectionTestResult> {
    let provider = create_provider(provider_config)?;
    provider.test_connection(model).await
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
