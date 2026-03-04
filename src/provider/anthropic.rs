use crate::{
    BabataResult,
    provider::{
        GenerationReqest, GenerationResponse, InteractionRequest, InteractionResponse, Model,
        Provider,
    },
};

use super::AnthropicCompatibleProvider;

#[derive(Debug)]
pub struct AnthropicProvider {
    inner: AnthropicCompatibleProvider,
}

const ANTHROPIC_SUPPORTED_MODELS: &[Model] = &[
    Model {
        provider: "anthropic",
        name: "claude-opus-4-6",
        context_length: 200_000,
    },
    Model {
        provider: "anthropic",
        name: "claude-sonnet-4-6",
        context_length: 200_000,
    },
];

impl AnthropicProvider {
    pub fn new(api_key: &str) -> Self {
        Self {
            inner: AnthropicCompatibleProvider::new(api_key, "https://api.anthropic.com"),
        }
    }
}

#[async_trait::async_trait]
impl Provider for AnthropicProvider {
    fn name() -> &'static str {
        "anthropic"
    }

    fn supported_models() -> &'static [Model] {
        ANTHROPIC_SUPPORTED_MODELS
    }

    async fn generate<'a>(
        &self,
        request: GenerationReqest<'a>,
    ) -> BabataResult<GenerationResponse> {
        self.inner.generate(request).await
    }

    async fn interact(&self, request: InteractionRequest) -> BabataResult<InteractionResponse> {
        self.inner.interact(request).await
    }
}
