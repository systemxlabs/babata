use crate::{
    BabataResult,
    provider::{
        GenerationRequest, GenerationResponse, InteractionRequest, InteractionResponse, Model,
        Provider,
    },
};

use super::OpenAICompatibleProvider;

#[derive(Debug)]
pub struct MiniMaxProvider {
    inner: OpenAICompatibleProvider,
}

const MINIMAX_SUPPORTED_MODELS: &[Model] = &[];

impl MiniMaxProvider {
    pub fn new(api_key: &str) -> Self {
        Self {
            inner: OpenAICompatibleProvider::new(api_key, "https://api.minimaxi.com/v1")
                .with_user_agent(None)
                .with_combined_system_prompt(true),
        }
    }
}

#[async_trait::async_trait]
impl Provider for MiniMaxProvider {
    fn name() -> &'static str {
        "minimax"
    }

    fn supported_models() -> &'static [Model] {
        MINIMAX_SUPPORTED_MODELS
    }

    async fn generate<'a>(
        &self,
        request: GenerationRequest<'a>,
    ) -> BabataResult<GenerationResponse> {
        self.inner.generate(request).await
    }

    async fn interact(&self, request: InteractionRequest) -> BabataResult<InteractionResponse> {
        self.inner.interact(request).await
    }
}
