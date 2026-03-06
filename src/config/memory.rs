use serde::{Deserialize, Serialize};

use crate::{BabataResult, error::BabataError};

#[derive(Default, Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MemoryConfig {
    #[default]
    Simple,
    Hybrid(HybridMemoryConfig),
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct HybridMemoryConfig {
    pub embedding: EmbeddingConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct LocalEmbeddingConfig {
    pub model: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct RemoteEmbeddingConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub dimension: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum EmbeddingConfig {
    Local(LocalEmbeddingConfig),
    Remote(RemoteEmbeddingConfig),
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        EmbeddingConfig::Local(LocalEmbeddingConfig {
            model: "baai/bge-m3".to_string(),
        })
    }
}

impl EmbeddingConfig {
    pub fn validate(&self) -> BabataResult<()> {
        match self {
            EmbeddingConfig::Local(config) => {
                if config.model.trim().is_empty() {
                    return Err(BabataError::config("Local embedding model cannot be empty"));
                }
                Ok(())
            }
            EmbeddingConfig::Remote(config) => {
                if config.api_key.trim().is_empty() {
                    return Err(BabataError::config("Embedding API key cannot be empty"));
                }
                if config.base_url.trim().is_empty() {
                    return Err(BabataError::config("Embedding base URL cannot be empty"));
                }
                if config.model.trim().is_empty() {
                    return Err(BabataError::config("Embedding model cannot be empty"));
                }
                if config.dimension == 0 {
                    return Err(BabataError::config(
                        "Embedding dimension must be greater than 0",
                    ));
                }
                Ok(())
            }
        }
    }

    pub fn embedding_name(&self) -> &str {
        match self {
            EmbeddingConfig::Local(config) => &config.model,
            EmbeddingConfig::Remote(config) => &config.model,
        }
    }

    pub fn matches_name(&self, name: &str) -> bool {
        self.embedding_name().eq_ignore_ascii_case(name)
    }
}
