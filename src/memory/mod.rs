mod embedding;
mod search;
mod store;

pub use embedding::{Embedder, ProviderEmbedder};
pub use search::{HybridSearch, MatchType, SearchResult};
pub use store::MemoryStore;

use crate::{BabataResult, config::Config, error::BabataError, message::Message};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct Memory {
    store: Arc<RwLock<MemoryStore>>,
    embedder: Arc<dyn Embedder>,
    config: MemoryConfig,
}

#[derive(Debug, Clone)]
pub struct MemoryConfig {
    pub bm25_weight: f32,
    pub vector_weight: f32,
    pub top_k_candidates: usize,
    pub final_results: usize,
    pub max_retries: usize,
    pub retry_delay_ms: u64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            bm25_weight: 0.5,
            vector_weight: 0.5,
            top_k_candidates: 30,
            final_results: 5,
            max_retries: 3,
            retry_delay_ms: 100,
        }
    }
}

impl Memory {
    pub fn new(embedder: Arc<dyn Embedder>) -> BabataResult<Self> {
        let dimension = embedder.dimension();
        let store = MemoryStore::new(dimension)?;

        Ok(Self {
            store: Arc::new(RwLock::new(store)),
            embedder,
            config: MemoryConfig::default(),
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

        // Retry logic for indexing
        let mut last_error = None;
        for attempt in 0..self.config.max_retries {
            match self.try_index_message(&role, message_type, &content).await {
                Ok(_) => return Ok(()),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.config.max_retries - 1 {
                        log::warn!(
                            "Failed to index message (attempt {}/{}), retrying...",
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

        Err(last_error.unwrap_or_else(|| BabataError::memory("Unknown indexing error")))
    }

    async fn try_index_message(
        &self,
        role: &str,
        message_type: &str,
        content: &str,
    ) -> BabataResult<()> {
        // Generate embedding first (outside of lock)
        let embedding = self.embedder.embed(content).await?;

        // Use write lock for transaction
        let mut store = self.store.write().await;
        store
            .insert_message_with_embedding(role, message_type, content, &embedding)
            .map(|_| ())
    }

    pub async fn search(&self, query: &str) -> BabataResult<Vec<SearchResult>> {
        let query_embedding = self.embedder.embed(query).await?;

        let store = self.store.read().await;
        let searcher =
            HybridSearch::new(&store, self.config.bm25_weight, self.config.vector_weight);

        searcher.search(
            query,
            &query_embedding,
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
    use crate::message::Content;

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
                format!("{}...", &result[..1000])
            } else {
                result.clone()
            }
        }
        _ => String::new(),
    }
}

pub fn build_memory(config: &Config) -> BabataResult<Memory> {
    let embedder = create_embedder(config)?;
    Memory::new(embedder)
}

fn create_embedder(config: &Config) -> BabataResult<Arc<dyn Embedder>> {
    if let Some(embedding_config) = config.get_embedding_config() {
        return Ok(Arc::new(ProviderEmbedder::new(
            embedding_config.api_key.clone(),
            embedding_config.base_url.clone(),
            embedding_config.model.clone(),
            embedding_config.dimension,
        )));
    }

    Err(BabataError::config(
        "No embedding configuration found. Please add 'embedding' section to config.json",
    ))
}
