use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct ProviderConfig {
    // The API key for authentication
    pub api_key: String,
}
