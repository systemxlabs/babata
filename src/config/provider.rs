use serde::{Deserialize, Serialize};

use crate::{BabataResult, error::BabataError};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "name")]
pub enum ProviderConfig {
    #[serde(rename = "openai")]
    OpenAI(OpenAIProviderConfig),
    #[serde(rename = "kimi")]
    Kimi(KimiProviderConfig),
    #[serde(rename = "moonshot")]
    Moonshot(MoonshotProviderConfig),
    #[serde(rename = "deepseek")]
    DeepSeek(DeepSeekProviderConfig),
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct OpenAIProviderConfig {
    pub api_key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct MoonshotProviderConfig {
    pub api_key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct KimiProviderConfig {
    pub api_key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct DeepSeekProviderConfig {
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
            ProviderConfig::Kimi(config) => &config.api_key,
            ProviderConfig::Moonshot(config) => &config.api_key,
            ProviderConfig::DeepSeek(config) => &config.api_key,
        }
    }

    pub fn provider_name(&self) -> &'static str {
        match self {
            ProviderConfig::OpenAI(_) => "openai",
            ProviderConfig::Kimi(_) => "kimi",
            ProviderConfig::Moonshot(_) => "moonshot",
            ProviderConfig::DeepSeek(_) => "deepseek",
        }
    }

    pub fn matches_name(&self, name: &str) -> bool {
        self.provider_name().eq_ignore_ascii_case(name)
    }
}
