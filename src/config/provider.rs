use serde::{Deserialize, Serialize};

use crate::{
    BabataResult,
    error::BabataError,
    provider::{
        AnthropicProvider, CustomProvider, DeepSeekProvider, KimiProvider, MiniMaxProvider,
        MoonshotProvider, OpenAIProvider, Provider,
    },
};

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
    #[serde(rename = "minimax")]
    MiniMax(MiniMaxProviderConfig),
    #[serde(rename = "anthropic")]
    Anthropic(AnthropicProviderConfig),
    #[serde(rename = "custom")]
    Custom(CustomProviderConfig),
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct MiniMaxProviderConfig {
    pub api_key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct AnthropicProviderConfig {
    pub api_key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct CustomProviderConfig {
    pub api_key: String,
    pub base_url: String,
    pub compatible_api: CompatibleApi,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CompatibleApi {
    Openai,
    Anthropic,
}

impl ProviderConfig {
    pub fn validate(&self) -> BabataResult<()> {
        let api_key = self.api_key().trim();
        if api_key.is_empty() {
            return Err(BabataError::config(
                "Provider api_key cannot be empty or whitespace",
            ));
        }

        if let ProviderConfig::Custom(config) = self {
            let base_url = config.base_url.trim();
            if base_url.is_empty() {
                return Err(BabataError::config(
                    "Custom provider base_url cannot be empty or whitespace",
                ));
            }
        }

        Ok(())
    }

    pub fn api_key(&self) -> &str {
        match self {
            ProviderConfig::OpenAI(config) => &config.api_key,
            ProviderConfig::Kimi(config) => &config.api_key,
            ProviderConfig::Moonshot(config) => &config.api_key,
            ProviderConfig::DeepSeek(config) => &config.api_key,
            ProviderConfig::MiniMax(config) => &config.api_key,
            ProviderConfig::Anthropic(config) => &config.api_key,
            ProviderConfig::Custom(config) => &config.api_key,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ProviderConfig::OpenAI(_) => OpenAIProvider::name(),
            ProviderConfig::Kimi(_) => KimiProvider::name(),
            ProviderConfig::Moonshot(_) => MoonshotProvider::name(),
            ProviderConfig::DeepSeek(_) => DeepSeekProvider::name(),
            ProviderConfig::MiniMax(_) => MiniMaxProvider::name(),
            ProviderConfig::Anthropic(_) => AnthropicProvider::name(),
            ProviderConfig::Custom(_) => CustomProvider::name(),
        }
    }

    pub fn matches_name(&self, name: &str) -> bool {
        self.name().eq_ignore_ascii_case(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_custom_provider_rejects_empty_base_url() {
        let config = ProviderConfig::Custom(CustomProviderConfig {
            api_key: "test-key".to_string(),
            base_url: "   ".to_string(),
            compatible_api: CompatibleApi::Openai,
        });

        let result = config.validate();
        assert!(result.is_err());
        let err = result.expect_err("expected base_url validation error");
        assert!(err.to_string().contains("base_url"));
    }

    #[test]
    fn parse_custom_provider_config_from_json() {
        let payload = r#"{
            "name": "custom",
            "api_key": "test-key",
            "base_url": "https://example.com/v1",
            "compatible_api": "openai"
        }"#;
        let parsed: ProviderConfig = serde_json::from_str(payload).expect("parse provider json");

        match parsed {
            ProviderConfig::Custom(config) => {
                assert_eq!(config.api_key, "test-key");
                assert_eq!(config.base_url, "https://example.com/v1");
                assert_eq!(config.compatible_api, CompatibleApi::Openai);
            }
            _ => panic!("expected ProviderConfig::Custom"),
        }
    }

    #[test]
    fn parse_minimax_provider_config_from_json() {
        let payload = r#"{
            "name": "minimax",
            "api_key": "test-key"
        }"#;
        let parsed: ProviderConfig = serde_json::from_str(payload).expect("parse provider json");

        match parsed {
            ProviderConfig::MiniMax(config) => {
                assert_eq!(config.api_key, "test-key");
            }
            _ => panic!("expected ProviderConfig::MiniMax"),
        }
    }
}
