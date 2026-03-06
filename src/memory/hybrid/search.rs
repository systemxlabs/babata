use super::store::{BM25Result, MemoryStore, VectorResult};
use crate::BabataResult;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub message_id: i64,
    pub content: String,
    pub score: f32,
    pub match_type: MatchType,
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchType {
    BM25,
    Vector,
    Hybrid,
}

pub struct HybridSearch<'a> {
    store: &'a MemoryStore,
    bm25_weight: f32,
    vector_weight: f32,
    rrf_k: f32,
}

impl<'a> HybridSearch<'a> {
    pub fn new(store: &'a MemoryStore, bm25_weight: f32, vector_weight: f32, rrf_k: f32) -> Self {
        Self {
            store,
            bm25_weight,
            vector_weight,
            rrf_k,
        }
    }

    pub fn search(
        &self,
        query_text: &str,
        query_embedding: &[f32],
        top_k_candidates: usize,
        final_results: usize,
    ) -> BabataResult<Vec<SearchResult>> {
        let bm25_results = self.store.bm25_search(query_text, top_k_candidates)?;

        let vector_results = self
            .store
            .vector_search(query_embedding, top_k_candidates)?;

        let fused = self.reciprocal_rank_fusion(&bm25_results, &vector_results, final_results);

        Ok(fused)
    }

    fn reciprocal_rank_fusion(
        &self,
        bm25_results: &[BM25Result],
        vector_results: &[VectorResult],
        limit: usize,
    ) -> Vec<SearchResult> {
        let mut scores: HashMap<i64, (f32, SearchResult)> = HashMap::new();

        for (rank, result) in bm25_results.iter().enumerate() {
            let rrf_score = self.bm25_weight / (self.rrf_k + rank as f32 + 1.0);

            let bonus = if rank < 3 {
                0.05
            } else if rank < 6 {
                0.02
            } else {
                0.0
            };

            scores.insert(
                result.message_id,
                (
                    rrf_score + bonus,
                    SearchResult {
                        message_id: result.message_id,
                        content: result.content.clone(),
                        score: rrf_score + bonus,
                        match_type: MatchType::BM25,
                        snippet: Some(result.snippet.clone()),
                    },
                ),
            );
        }

        for (rank, result) in vector_results.iter().enumerate() {
            let rrf_score = self.vector_weight / (self.rrf_k + rank as f32 + 1.0);

            scores
                .entry(result.message_id)
                .and_modify(|(score, hybrid_result)| {
                    *score += rrf_score;
                    hybrid_result.score = *score;
                    hybrid_result.match_type = MatchType::Hybrid;
                })
                .or_insert((
                    rrf_score,
                    SearchResult {
                        message_id: result.message_id,
                        content: result.content.clone(),
                        score: rrf_score,
                        match_type: MatchType::Vector,
                        snippet: None,
                    },
                ));
        }

        let mut results: Vec<_> = scores.into_values().collect();
        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        results
            .into_iter()
            .take(limit)
            .map(|(_, result)| result)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::hybrid::store::MemoryStore;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn temp_db_path() -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!(
            "babata_search_test_{}_{}_{}.db",
            std::process::id(),
            id,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    #[ignore]
    fn test_hybrid_search() -> BabataResult<()> {
        use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

        let mut model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::BGEM3)
                .with_cache_dir(std::env::temp_dir().join("fastembed_cache")),
        )
        .expect("Failed to create embedding model");

        let dimension = 1024;
        let mut store = MemoryStore::new(temp_db_path(), dimension)?;

        let texts = ["人工智能技术发展", "机器学习算法", "深度学习神经网络"];
        let embeddings = model.embed(texts, None).unwrap();

        for (text, embedding) in texts.iter().zip(embeddings.iter()) {
            store.insert_message("user", "text", text, embedding)?;
        }

        let hybrid = HybridSearch::new(&store, 1.0, 1.0, 60.0);

        let query_embedding = model.embed(vec!["人工智能"], None).unwrap();
        let results = hybrid.search("人工智能", &query_embedding[0], 10, 5)?;

        assert!(!results.is_empty(), "Should find hybrid search results");

        Ok(())
    }

    #[test]
    #[ignore]
    fn test_hybrid_search_match_type() -> BabataResult<()> {
        use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

        let mut model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::BGEM3)
                .with_cache_dir(std::env::temp_dir().join("fastembed_cache")),
        )
        .expect("Failed to create embedding model");

        let dimension = 1024;
        let mut store = MemoryStore::new(temp_db_path(), dimension)?;

        let texts = ["Rust编程语言", "Python编程"];
        let embeddings = model.embed(texts, None).unwrap();

        for (text, embedding) in texts.iter().zip(embeddings.iter()) {
            store.insert_message("user", "text", text, embedding)?;
        }

        let hybrid = HybridSearch::new(&store, 1.0, 1.0, 60.0);

        let query_embedding = model.embed(vec!["Rust编程"], None).unwrap();
        let results = hybrid.search("Rust编程", &query_embedding[0], 10, 5)?;

        if !results.is_empty() {
            let first = &results[0];
            assert!(
                first.match_type == MatchType::Hybrid || first.match_type == MatchType::BM25,
                "First result should be Hybrid or BM25"
            );
        }

        Ok(())
    }

    #[test]
    #[ignore]
    fn test_hybrid_search_weights() -> BabataResult<()> {
        use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

        let mut model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::BGEM3)
                .with_cache_dir(std::env::temp_dir().join("fastembed_cache")),
        )
        .expect("Failed to create embedding model");

        let dimension = 1024;
        let mut store = MemoryStore::new(temp_db_path(), dimension)?;

        let embedding = model.embed(vec!["测试消息"], None).unwrap();
        store.insert_message("user", "text", "测试消息", &embedding[0])?;

        let hybrid_bm25_heavy = HybridSearch::new(&store, 2.0, 0.5, 60.0);
        let hybrid_vector_heavy = HybridSearch::new(&store, 0.5, 2.0, 60.0);

        let results1 = hybrid_bm25_heavy.search("测试", &embedding[0], 10, 5)?;
        let results2 = hybrid_vector_heavy.search("测试", &embedding[0], 10, 5)?;

        assert!(!results1.is_empty());
        assert!(!results2.is_empty());

        Ok(())
    }

    #[test]
    fn test_reciprocal_rank_fusion() -> BabataResult<()> {
        let dimension = 1024;
        let store = MemoryStore::new(temp_db_path(), dimension)?;
        let hybrid = HybridSearch::new(&store, 1.0, 1.0, 60.0);

        let bm25_results = vec![
            BM25Result {
                message_id: 1,
                content: "First BM25".to_string(),
                score: -5.0,
                snippet: "First".to_string(),
            },
            BM25Result {
                message_id: 2,
                content: "Second BM25".to_string(),
                score: -4.0,
                snippet: "Second".to_string(),
            },
        ];

        let vector_results = vec![
            VectorResult {
                message_id: 2,
                content: "Second BM25".to_string(),
                distance: 0.1,
            },
            VectorResult {
                message_id: 3,
                content: "Third Vector".to_string(),
                distance: 0.2,
            },
        ];

        let fused = hybrid.reciprocal_rank_fusion(&bm25_results, &vector_results, 5);

        assert!(!fused.is_empty());

        let msg2_result = fused.iter().find(|r| r.message_id == 2);
        assert!(msg2_result.is_some());
        assert_eq!(msg2_result.unwrap().match_type, MatchType::Hybrid);

        Ok(())
    }
}
