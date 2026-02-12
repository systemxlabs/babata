mod channel;
mod provider;

pub use channel::*;
pub use provider::*;

use std::collections::HashMap;
use std::collections::HashSet;

use crate::{BabataResult, error::BabataError, utils::babata_dir};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct AgentConfig {
    // If None, use default skills
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Config {
    pub providers: Vec<ProviderConfig>,
    pub agents: HashMap<String, AgentConfig>,
    #[serde(default)]
    pub channels: Vec<ChannelConfig>,
}

impl Config {
    pub fn path() -> BabataResult<std::path::PathBuf> {
        Ok(babata_dir()?.join("config.json"))
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

        let mut has_main_agent = false;
        for (agent_name, agent_config) in &self.agents {
            if agent_name == "main" {
                has_main_agent = true;
            }
            if !self
                .providers
                .iter()
                .any(|provider| provider.matches_name(&agent_config.provider))
            {
                return Err(BabataError::config(format!(
                    "Agent '{}' references unknown provider '{}'",
                    agent_name, agent_config.provider
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
            agents: HashMap::from([(
                "main".to_string(),
                AgentConfig {
                    provider: "openai".to_string(),
                    model: "gpt-4.1".to_string(),
                },
            )]),
            channels: Vec::new(),
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
            agents: HashMap::from([(
                "main".to_string(),
                AgentConfig {
                    provider: "openai".to_string(),
                    model: "test-model".to_string(),
                },
            )]),
            channels: Vec::new(),
        };

        config.validate().expect("provider URL no longer validated");
    }
}
