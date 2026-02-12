use serde::{Deserialize, Serialize};

use crate::{BabataResult, error::BabataError};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type")]
pub enum ProviderConfig {
    #[serde(rename = "openai")]
    OpenAI(OpenAIProviderConfig),
    #[serde(rename = "moonshot")]
    Moonshot(MoonshotProviderConfig),
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct OpenAIProviderConfig {
    pub api_key: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct MoonshotProviderConfig {
    pub api_key: String,
}

impl ProviderConfig {
    pub fn validate(&self) -> BabataResult<()> {
        let api_key = self.api_key().trim();
        if api_key.is_empty() {
            return Err(BabataError::config(
                "Provider api_key cannot be empty or whitespace",
            ));
        }
        Ok(())
    }

    pub fn api_key(&self) -> &str {
        match self {
            ProviderConfig::OpenAI(config) => &config.api_key,
            ProviderConfig::Moonshot(config) => &config.api_key,
        }
    }

    pub fn provider_name(&self) -> &'static str {
        match self {
            ProviderConfig::OpenAI(_) => "openai",
            ProviderConfig::Moonshot(_) => "moonshot",
        }
    }

    pub fn matches_name(&self, name: &str) -> bool {
        self.provider_name().eq_ignore_ascii_case(name)
    }
}
