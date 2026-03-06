use crate::BabataResult;
use crate::error::BabataError;
use crate::utils::babata_dir;
use async_trait::async_trait;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::{path::PathBuf, str::FromStr};
use tokio::sync::Mutex;

#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, texts: &[&str]) -> BabataResult<Vec<f32>>;
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
    pub fn new_qwen(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string(),
            model: "text-embedding-v3".to_string(),
            dimension: 1024,
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
    async fn embed(&self, texts: &[&str]) -> BabataResult<Vec<f32>> {
        let response = self
            .client
            .post(format!("{}/embeddings", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&serde_json::json!({
                "model": self.model,
                "input": texts,
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

pub struct LocalEmbedder {
    model: Mutex<TextEmbedding>,
    dimension: usize,
}

impl LocalEmbedder {
    pub fn new(model_name: &str) -> BabataResult<Self> {
        let cache_dir = babata_dir()?.join("models/embedding");
        Self::new_with_cache_dir(model_name, cache_dir)
    }

    pub fn new_with_cache_dir(model_name: &str, cache_dir: PathBuf) -> BabataResult<Self> {
        let model = EmbeddingModel::from_str(model_name).map_err(|e| {
            BabataError::internal(format!("Invalid model name '{}': {}", model_name, e))
        })?;
        let dimension = TextEmbedding::get_model_info(&model).unwrap().dim;

        let embedding_model = TextEmbedding::try_new(
            InitOptions::new(model).with_cache_dir(cache_dir),
        )
        .map_err(|e| {
            BabataError::internal(format!("Failed to initialize local embedding model: {}", e))
        })?;

        Ok(Self {
            model: Mutex::new(embedding_model),
            dimension,
        })
    }
}

#[async_trait]
impl Embedder for LocalEmbedder {
    async fn embed(&self, texts: &[&str]) -> BabataResult<Vec<f32>> {
        let embeddings =
            self.model.lock().await.embed(texts, None).map_err(|e| {
                BabataError::internal(format!("Failed to generate embedding: {}", e))
            })?;

        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| BabataError::internal("No embedding returned"))
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_qwen_embedding() {
        let api_key = std::env::var("DASHSCOPE_API_KEY")
            .expect("DASHSCOPE_API_KEY environment variable not set");

        let embedder = ProviderEmbedder::new_qwen(api_key);
        assert_eq!(embedder.dimension(), 1024);

        // Test basic English text
        let text_en = "Hello, world!";
        let embedding_en = embedder
            .embed(&[text_en])
            .await
            .expect("Failed to get embedding");

        assert_eq!(embedding_en.len(), 1024);

        // Test Chinese text
        let text_cn = "你好，世界！";
        let embedding_cn = embedder
            .embed(&[text_cn])
            .await
            .expect("Failed to get Chinese embedding");

        assert_eq!(embedding_cn.len(), 1024);

        // Test long text
        let text_long = "This is a longer text that contains multiple sentences. \
                         It is used to test the embedding API with more substantial content. \
                         The embedding should still work correctly regardless of text length, \
                         as long as it's within the model's token limit.";

        let embedding_long = embedder
            .embed(&[text_long])
            .await
            .expect("Failed to get long text embedding");

        assert_eq!(embedding_long.len(), 1024);
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        dot_product / (norm_a * norm_b)
    }

    #[tokio::test]
    #[ignore]
    async fn test_qwen_embedding_similarity() {
        let api_key = std::env::var("DASHSCOPE_API_KEY")
            .expect("DASHSCOPE_API_KEY environment variable not set");

        let embedder = ProviderEmbedder::new_qwen(api_key);

        let text1 = "The cat sits on the mat";
        let text2 = "A cat is sitting on a mat";
        let text3 = "The weather is nice today";

        let emb1 = embedder
            .embed(&[text1])
            .await
            .expect("Failed to get embedding 1");
        let emb2 = embedder
            .embed(&[text2])
            .await
            .expect("Failed to get embedding 2");
        let emb3 = embedder
            .embed(&[text3])
            .await
            .expect("Failed to get embedding 3");

        let sim_12 = cosine_similarity(&emb1, &emb2);
        let sim_13 = cosine_similarity(&emb1, &emb3);

        println!("Similarity between similar texts: {}", sim_12);
        println!("Similarity between different texts: {}", sim_13);

        // Similar texts should have higher similarity than different texts
        assert!(
            sim_12 > sim_13,
            "Similar texts should have higher similarity. Got sim_12={}, sim_13={}",
            sim_12,
            sim_13
        );
    }

    #[tokio::test]
    #[ignore]
    async fn test_local_embedding() {
        let embedder = LocalEmbedder::new("baai/bge-m3").expect("Failed to create local embedder");

        let text = "Hello, world!";
        let embedding = embedder
            .embed(&[text])
            .await
            .expect("Failed to get embedding");

        assert_eq!(embedding.len(), embedder.dimension());
        assert!(
            embedding.iter().any(|&x| x != 0.0),
            "Embedding should not be all zeros"
        );
    }

    #[tokio::test]
    #[ignore]
    async fn test_local_embedding_similarity() {
        let embedder = LocalEmbedder::new("baai/bge-m3").expect("Failed to create local embedder");

        let text1 = "The cat sits on the mat";
        let text2 = "A cat is sitting on a mat";
        let text3 = "The weather is nice today";

        let emb1 = embedder
            .embed(&[text1])
            .await
            .expect("Failed to get embedding 1");
        let emb2 = embedder
            .embed(&[text2])
            .await
            .expect("Failed to get embedding 2");
        let emb3 = embedder
            .embed(&[text3])
            .await
            .expect("Failed to get embedding 3");

        let sim_12 = cosine_similarity(&emb1, &emb2);
        let sim_13 = cosine_similarity(&emb1, &emb3);

        println!("Similarity between similar texts: {}", sim_12);
        println!("Similarity between different texts: {}", sim_13);

        assert!(
            sim_12 > sim_13,
            "Similar texts should have higher similarity. Got sim_12={}, sim_13={}",
            sim_12,
            sim_13
        );
    }
}
