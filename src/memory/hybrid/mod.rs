mod embedding;
mod search;
mod store;

pub use embedding::{Embedder, LocalEmbedder, ProviderEmbedder};
pub use search::{HybridSearch, SearchResult};
pub use store::MemoryStore;

use crate::{
    BabataResult,
    config::EmbeddingConfig,
    error::BabataError,
    memory::Memory,
    message::{Content, Message},
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct HybridMemory {
    store: Arc<Mutex<MemoryStore>>,
    embedder: Arc<dyn Embedder>,
    config: HybridMemoryConfig,
}

#[derive(Debug, Clone)]
pub struct HybridMemoryConfig {
    pub bm25_weight: f32,
    pub vector_weight: f32,
    pub rrf_k: f32,
    pub top_k_candidates: usize,
    pub final_results: usize,
    pub max_retries: usize,
    pub retry_delay_ms: u64,
}

impl Default for HybridMemoryConfig {
    fn default() -> Self {
        Self {
            bm25_weight: 0.6,
            vector_weight: 0.4,
            rrf_k: 60.0,
            top_k_candidates: 40,
            final_results: 10,
            max_retries: 3,
            retry_delay_ms: 100,
        }
    }
}

impl HybridMemory {
    pub fn new(embedder: Arc<dyn Embedder>) -> BabataResult<Self> {
        let dimension = embedder.dimension();
        let db_path = MemoryStore::default_db_path()?;
        let store = MemoryStore::new(db_path, dimension)?;

        Ok(Self {
            store: Arc::new(Mutex::new(store)),
            embedder,
            config: HybridMemoryConfig::default(),
        })
    }

    pub async fn index_message(&self, message: &Message) -> BabataResult<()> {
        let content = extract_text_from_message(message);
        if content.is_empty() {
            return Ok(());
        }

        let role = message.role().to_string();
        let message_type = match message {
            Message::UserPrompt { .. } => "user_prompt",
            Message::AssistantResponse { .. } => "assistant_response",
            Message::AssistantToolCalls { .. } => "assistant_tool_calls",
            Message::ToolResult { .. } => "tool_result",
        };

        // Generate embedding first (outside of lock)
        let mut last_error = None;
        for attempt in 0..self.config.max_retries {
            match self.embedder.embed(&[content.as_str()]).await {
                Ok(embeddings) => {
                    let embedding = embeddings
                        .first()
                        .ok_or(BabataError::memory("No embedding returned"))?;
                    // Now acquire lock and insert
                    let mut store = self.store.lock().await;
                    return store
                        .insert_message(&role, message_type, &content, embedding)
                        .map(|_| ());
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.config.max_retries - 1 {
                        log::warn!(
                            "Failed to generate embedding (attempt {}/{}), retrying...",
                            attempt + 1,
                            self.config.max_retries
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(
                            self.config.retry_delay_ms * (attempt as u64 + 1),
                        ))
                        .await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| BabataError::memory("Unknown embedding error")))
    }

    pub async fn search(&self, query: &str) -> BabataResult<Vec<SearchResult>> {
        let embeddings = self.embedder.embed(&[query]).await?;
        let query_embedding = embeddings
            .first()
            .ok_or(BabataError::memory("No embedding returned"))?;

        let store = self.store.lock().await;
        let searcher = HybridSearch::new(
            &store,
            self.config.bm25_weight,
            self.config.vector_weight,
            self.config.rrf_k,
        );

        searcher.search(
            query,
            query_embedding,
            self.config.top_k_candidates,
            self.config.final_results,
        )
    }

    pub async fn get_context_for_prompt(&self, query: &str) -> BabataResult<String> {
        let results = self.search(query).await?;

        if results.is_empty() {
            return Ok(String::new());
        }

        let mut context = String::from("## Relevant Memory\n\n");

        for (i, result) in results.iter().enumerate() {
            context.push_str(&format!(
                "[{}] (relevance: {:.2}, type: {:?})\n{}\n\n",
                i + 1,
                result.score,
                result.match_type,
                result.snippet.as_ref().unwrap_or(&result.content)
            ));
        }

        Ok(context)
    }
}

fn extract_text_from_message(message: &Message) -> String {
    match message {
        Message::UserPrompt { content } | Message::AssistantResponse { content, .. } => content
            .iter()
            .filter_map(|c| match c {
                Content::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Message::ToolResult { result, .. } => {
            if result.len() > 1000 {
                result.chars().take(1000).collect::<String>() + "..."
            } else {
                result.clone()
            }
        }
        _ => String::new(),
    }
}

pub fn build_memory(
    hybrid_config: &crate::config::HybridMemoryConfig,
) -> BabataResult<HybridMemory> {
    let embedder = create_embedder(&hybrid_config.embedding)?;
    HybridMemory::new(embedder)
}

fn create_embedder(embedding: &EmbeddingConfig) -> BabataResult<Arc<dyn Embedder>> {
    match embedding {
        EmbeddingConfig::Local(local_config) => {
            let embedder = LocalEmbedder::new(&local_config.model)?;
            Ok(Arc::new(embedder))
        }
        EmbeddingConfig::Remote(remote_config) => Ok(Arc::new(ProviderEmbedder::new(
            remote_config.api_key.clone(),
            remote_config.base_url.clone(),
            remote_config.model.clone(),
            remote_config.dimension,
        ))),
    }
}

impl std::fmt::Debug for HybridMemory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HybridMemory")
            .field("config", &self.config)
            .finish()
    }
}

#[async_trait::async_trait]
impl Memory for HybridMemory {
    fn name() -> &'static str {
        "hybrid"
    }

    async fn append_messages(&self, messages: Vec<Message>) -> BabataResult<()> {
        for message in &messages {
            self.index_message(message).await?;
        }
        Ok(())
    }

    async fn build_context(&self, prompt: &[Content]) -> BabataResult<String> {
        let query = extract_query_from_messages(prompt);
        if query.is_empty() {
            return Ok(String::new());
        }

        let context = self.get_context_for_prompt(&query).await?;
        Ok(context)
    }
}

fn extract_query_from_messages(prompt: &[Content]) -> String {
    let text: String = prompt
        .iter()
        .filter_map(|c| match c {
            Content::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ");

    if !text.is_empty() {
        // Limit query length to avoid excessive search time
        const MAX_QUERY_LENGTH: usize = 200;
        if text.len() > MAX_QUERY_LENGTH {
            return text.chars().take(MAX_QUERY_LENGTH).collect();
        }
        return text;
    }

    String::new()
}
