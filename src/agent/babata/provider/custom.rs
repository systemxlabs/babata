use crate::{
    BabataResult,
    agent::babata::{
        AnthropicCompatibleProvider, GenerationRequest, GenerationResponse, InteractionRequest,
        InteractionResponse, Model, OpenAICompatibleProvider, Provider,
    },
    config::{CompatibleApi, CustomProviderConfig},
};

#[derive(Debug)]
enum CustomProviderInner {
    OpenAI(OpenAICompatibleProvider),
    Anthropic(AnthropicCompatibleProvider),
}

#[derive(Debug)]
pub struct CustomProvider {
    inner: CustomProviderInner,
}

const CUSTOM_SUPPORTED_MODELS: &[Model] = &[];

impl CustomProvider {
    pub fn new(config: &CustomProviderConfig) -> Self {
        let inner = match config.compatible_api {
            CompatibleApi::Openai => CustomProviderInner::OpenAI(
                OpenAICompatibleProvider::new(&config.api_key, &config.base_url)
                    .with_user_agent(None),
            ),
            CompatibleApi::Anthropic => CustomProviderInner::Anthropic(
                AnthropicCompatibleProvider::new(&config.api_key, &config.base_url),
            ),
        };

        Self { inner }
    }
}

#[async_trait::async_trait]
impl Provider for CustomProvider {
    fn name() -> &'static str {
        "custom"
    }

    fn supported_models() -> &'static [Model] {
        CUSTOM_SUPPORTED_MODELS
    }

    async fn generate<'a>(
        &self,
        request: GenerationRequest<'a>,
    ) -> BabataResult<GenerationResponse> {
        match &self.inner {
            CustomProviderInner::OpenAI(provider) => provider.generate(request).await,
            CustomProviderInner::Anthropic(provider) => provider.generate(request).await,
        }
    }

    async fn interact(&self, request: InteractionRequest) -> BabataResult<InteractionResponse> {
        match &self.inner {
            CustomProviderInner::OpenAI(provider) => provider.interact(request).await,
            CustomProviderInner::Anthropic(provider) => provider.interact(request).await,
        }
    }
}
