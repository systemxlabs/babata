use crate::{
    BabataResult,
    provider::{
        GenerationReqest, GenerationResponse, InteractionRequest, InteractionResponse, Provider,
    },
};

use super::OpenAIProvider;

#[derive(Debug)]
pub struct MoonshotProvider {
    inner: OpenAIProvider,
}

impl MoonshotProvider {
    pub fn new(api_key: &str, model: &str) -> Self {
        let inner = OpenAIProvider::new(api_key, model).with_base_url("https://api.moonshot.cn/v1");
        Self { inner }
    }
}

#[async_trait::async_trait]
impl Provider for MoonshotProvider {
    fn name() -> &'static str {
        "Moonshot"
    }

    fn model(&self) -> &str {
        self.inner.model()
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
