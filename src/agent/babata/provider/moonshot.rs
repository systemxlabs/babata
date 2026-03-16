use crate::{
    BabataResult,
    agent::babata::{
        GenerationRequest, GenerationResponse, InteractionRequest, InteractionResponse, Model,
        Provider,
    },
};

use super::OpenAICompatibleProvider;

#[derive(Debug)]
pub struct MoonshotProvider {
    inner: OpenAICompatibleProvider,
}

const MOONSHOT_SUPPORTED_MODELS: &[Model] = &[Model {
    provider: "moonshot",
    name: "kimi-k2.5",
    context_length: 128_000,
}];

impl MoonshotProvider {
    pub fn new(api_key: &str) -> Self {
        let inner = OpenAICompatibleProvider::new(api_key, "https://api.moonshot.cn/v1")
            .with_user_agent(None);
        Self { inner }
    }
}

#[async_trait::async_trait]
impl Provider for MoonshotProvider {
    fn name() -> &'static str {
        "moonshot"
    }

    fn supported_models() -> &'static [Model] {
        MOONSHOT_SUPPORTED_MODELS
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
