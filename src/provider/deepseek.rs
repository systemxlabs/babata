use crate::{
    BabataResult,
    provider::{
        GenerationReqest, GenerationResponse, InteractionRequest, InteractionResponse, Provider,
    },
};

use super::OpenAIProvider;

#[derive(Debug)]
pub struct DeepSeekProvider {
    inner: OpenAIProvider,
}

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

    fn supported_models() -> &'static [&'static str] {
        &["deepseek-chat", "deepseek-reasoner"]
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
