use crate::{
    BabataResult,
    provider::{
        GenerationReqest, GenerationResponse, InteractionRequest, InteractionResponse, Model,
        Provider,
    },
};

use super::OpenAIProvider;

#[derive(Debug)]
pub struct DeepSeekProvider {
    inner: OpenAIProvider,
}

const DEEPSEEK_SUPPORTED_MODELS: &[Model] = &[
    Model {
        provider: "deepseek",
        name: "deepseek-chat",
        context_length: 64_000,
    },
    Model {
        provider: "deepseek",
        name: "deepseek-reasoner",
        context_length: 64_000,
    },
];

impl DeepSeekProvider {
    pub fn new(api_key: &str) -> Self {
        let inner = OpenAIProvider::new(api_key).with_base_url("https://api.deepseek.com/v1");
        Self { inner }
    }
}

#[async_trait::async_trait]
impl Provider for DeepSeekProvider {
    fn name() -> &'static str {
        "deepseek"
    }

    fn supported_models() -> &'static [Model] {
        DEEPSEEK_SUPPORTED_MODELS
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
