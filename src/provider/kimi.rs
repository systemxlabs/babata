use crate::{
    BabataResult,
    provider::{
        GenerationRequest, GenerationResponse, InteractionRequest, InteractionResponse, Model,
        Provider,
    },
};

use super::OpenAICompatibleProvider;

#[derive(Debug)]
pub struct KimiProvider {
    inner: OpenAICompatibleProvider,
}

const KIMI_SUPPORTED_MODELS: &[Model] = &[Model {
    provider: "kimi",
    name: "kimi-k2.5",
    context_length: 128_000,
}];

impl KimiProvider {
    pub fn new(api_key: &str) -> Self {
        let inner = OpenAICompatibleProvider::new(api_key, "https://api.kimi.com/coding/v1")
            .with_user_agent(Some("KimiCLI/1.6".to_string()));
        Self { inner }
    }
}

#[async_trait::async_trait]
impl Provider for KimiProvider {
    fn name() -> &'static str {
        "kimi"
    }

    fn supported_models() -> &'static [Model] {
        KIMI_SUPPORTED_MODELS
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
