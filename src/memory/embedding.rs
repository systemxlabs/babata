use crate::BabataResult;
use crate::error::BabataError;
use async_trait::async_trait;

#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, text: &str) -> BabataResult<Vec<f32>>;
    fn dimension(&self) -> usize;
}

pub struct ProviderEmbedder {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    model: String,
    dimension: usize,
}

impl ProviderEmbedder {
    pub fn new_openai(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
            model: "text-embedding-3-small".to_string(),
            dimension: 1536,
        }
    }

    pub fn new_deepseek(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            base_url: "https://api.deepseek.com/v1".to_string(),
            model: "deepseek-embedding".to_string(),
            dimension: 1536,
        }
    }

    pub fn new(api_key: String, base_url: String, model: String, dimension: usize) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            base_url,
            model,
            dimension,
        }
    }
}

#[async_trait]
impl Embedder for ProviderEmbedder {
    async fn embed(&self, text: &str) -> BabataResult<Vec<f32>> {
        let response = self
            .client
            .post(format!("{}/embeddings", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&serde_json::json!({
                "model": self.model,
                "input": text,
            }))
            .send()
            .await
            .map_err(|e| BabataError::internal(format!("Failed to call embedding API: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(BabataError::internal(format!(
                "Embedding API returned error {}: {}",
                status, error_text
            )));
        }

        let json: serde_json::Value = response.json().await.map_err(|e| {
            BabataError::internal(format!("Failed to parse embedding response: {}", e))
        })?;

        let embedding = json["data"][0]["embedding"]
            .as_array()
            .ok_or_else(|| BabataError::internal("Invalid embedding response format"))?
            .iter()
            .filter_map(|v| v.as_f64().map(|f| f as f32))
            .collect();

        Ok(embedding)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}
