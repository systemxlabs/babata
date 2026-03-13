use crate::{
    BabataResult,
    agent::babata::{
        GenerationRequest, GenerationResponse, InteractionRequest, InteractionResponse, Model,
        Provider,
    },
};

use super::OpenAICompatibleProvider;

#[derive(Debug)]
pub struct OpenAIProvider {
    inner: OpenAICompatibleProvider,
}

const OPENAI_SUPPORTED_MODELS: &[Model] = &[Model {
    provider: "openai",
    name: "gpt-4.1",
    context_length: 1_000_000,
}];

impl OpenAIProvider {
    pub fn new(api_key: &str) -> Self {
        Self {
            inner: OpenAICompatibleProvider::new(api_key, "https://api.openai.com/v1")
                .with_user_agent(None),
        }
    }
}

#[async_trait::async_trait]
impl Provider for OpenAIProvider {
    fn name() -> &'static str {
        "openai"
    }

    fn supported_models() -> &'static [Model] {
        OPENAI_SUPPORTED_MODELS
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
