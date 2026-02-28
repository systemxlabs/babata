mod channel;
mod provider;

pub use channel::*;
pub use provider::*;

use std::collections::HashSet;

use crate::{BabataResult, error::BabataError, utils::babata_dir};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct EmbeddingConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub dimension: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct AgentConfig {
    pub name: String,
    // If None, use default skills
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Config {
    pub providers: Vec<ProviderConfig>,
    pub agents: Vec<AgentConfig>,
    #[serde(default)]
    pub channels: Vec<ChannelConfig>,
    #[serde(default)]
    pub embedding: Option<EmbeddingConfig>,
}

impl Config {
    pub fn path() -> BabataResult<std::path::PathBuf> {
        Ok(babata_dir()?.join("config.json"))
    }

    pub fn load_or_init() -> BabataResult<Self> {
        let config_path = Self::path()?;
        if config_path.exists() {
            Self::load()
        } else {
            Ok(Self {
                providers: Vec::new(),
                agents: Vec::new(),
                channels: Vec::new(),
                embedding: None,
            })
        }
    }

    pub fn load() -> BabataResult<Self> {
        let config_path = Self::path()?;
        let raw = std::fs::read_to_string(&config_path).map_err(|err| {
            BabataError::config(format!(
                "Failed to read config file '{}': {}",
                config_path.display(),
                err
            ))
        })?;
        let config = serde_json::from_str::<Config>(&raw).map_err(|err| {
            BabataError::config(format!(
                "Failed to parse config file '{}': {}",
                config_path.display(),
                err
            ))
        })?;
        config.validate()?;
        Ok(config)
    }

    pub fn save(&self) -> BabataResult<()> {
        let config_path = Self::path()?;
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                BabataError::config(format!(
                    "Failed to create config directory '{}': {}",
                    parent.display(),
                    err
                ))
            })?;
        }

        let payload = serde_json::to_string_pretty(self)
            .map_err(|err| BabataError::config(format!("Failed to serialize config: {}", err)))?;

        std::fs::write(&config_path, payload).map_err(|err| {
            BabataError::config(format!(
                "Failed to write config file '{}': {}",
                config_path.display(),
                err
            ))
        })?;

        Ok(())
    }

    pub fn validate(&self) -> BabataResult<()> {
        let mut provider_names = HashSet::new();
        for provider in &self.providers {
            provider.validate()?;
            let normalized_name = provider.provider_name().to_string();
            if !provider_names.insert(normalized_name) {
                return Err(BabataError::config(format!(
                    "Duplicate provider type '{}' found in configuration",
                    provider.provider_name()
                )));
            }
        }

        let mut agent_names = HashSet::new();
        let mut has_main_agent = false;
        for agent_config in &self.agents {
            if agent_config.name.trim().is_empty() {
                return Err(BabataError::config("Agent name cannot be empty"));
            }

            if !agent_names.insert(agent_config.name.clone()) {
                return Err(BabataError::config(format!(
                    "Duplicate agent name '{}' found in configuration",
                    agent_config.name
                )));
            }

            if agent_config.name == "main" {
                has_main_agent = true;
            }
            if !self
                .providers
                .iter()
                .any(|provider| provider.matches_name(&agent_config.provider))
            {
                return Err(BabataError::config(format!(
                    "Agent '{}' references unknown provider '{}'",
                    agent_config.name, agent_config.provider
                )));
            }
        }
        if !has_main_agent {
            return Err(BabataError::config(
                "No 'main' agent defined in configuration",
            ));
        }

        for channel in &self.channels {
            match channel {
                ChannelConfig::Telegram(telegram) => telegram.validate()?,
            }
        }

        Ok(())
    }

    pub fn upsert_provider(&mut self, provider_config: ProviderConfig) {
        if let Some(existing) = self
            .providers
            .iter_mut()
            .find(|existing| existing.matches_name(provider_config.provider_name()))
        {
            *existing = provider_config;
            return;
        }

        self.providers.push(provider_config);
    }

    pub fn upsert_channel(&mut self, channel_config: ChannelConfig) {
        if let Some(existing) = self.channels.iter_mut().find(|existing| {
            matches!(
                (existing, &channel_config),
                (ChannelConfig::Telegram(_), ChannelConfig::Telegram(_))
            )
        }) {
            *existing = channel_config;
            return;
        }

        self.channels.push(channel_config);
    }

    pub fn upsert_agent(&mut self, agent_config: AgentConfig) {
        if let Some(existing) = self
            .agents
            .iter_mut()
            .find(|existing| existing.name == agent_config.name)
        {
            *existing = agent_config;
            return;
        }

        self.agents.push(agent_config);
    }

    pub fn get_agent(&self, agent_name: &str) -> Option<&AgentConfig> {
        self.agents.iter().find(|agent| agent.name == agent_name)
    }

    pub fn get_provider(&self, provider_name: &str) -> Option<&ProviderConfig> {
        self.providers
            .iter()
            .find(|provider| provider.matches_name(provider_name))
    }

    pub fn get_embedding_config(&self) -> Option<&EmbeddingConfig> {
        self.embedding.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_json_roundtrip() {
        let config = Config {
            providers: vec![ProviderConfig::OpenAI(OpenAIProviderConfig {
                api_key: "test-api-key".to_string(),
            })],
            agents: vec![AgentConfig {
                name: "main".to_string(),
                provider: "openai".to_string(),
                model: "gpt-4.1".to_string(),
            }],
            channels: Vec::new(),
            embedding: None,
        };

        let json = serde_json::to_string(&config).expect("serialize config to json");
        let parsed: Config = serde_json::from_str(&json).expect("deserialize config from json");

        assert_eq!(config, parsed);
    }

    #[test]
    fn validate_rejects_invalid_provider_url() {
        let config = Config {
            providers: vec![ProviderConfig::OpenAI(OpenAIProviderConfig {
                api_key: "test-api-key".to_string(),
            })],
            agents: vec![AgentConfig {
                name: "main".to_string(),
                provider: "openai".to_string(),
                model: "test-model".to_string(),
            }],
            channels: Vec::new(),
            embedding: None,
        };

        config.validate().expect("provider URL no longer validated");
    }
}
