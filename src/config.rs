use std::collections::HashMap;

use crate::{BabataResult, error::BabataError, utils::babata_dir};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct AgentConfig {
    // If None, use default skills
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct ProviderConfig {
    // The completed URL for the provider's API
    pub base_url: String,
    // The API key for authentication
    pub api_key: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Config {
    pub providers: HashMap<String, ProviderConfig>,
    pub agents: HashMap<String, AgentConfig>,
}

impl Config {
    pub fn load() -> BabataResult<Self> {
        let config_path = babata_dir()?.join("config.json");
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

    pub fn validate(&self) -> BabataResult<()> {
        for (provider_name, provider_config) in &self.providers {
            let parsed = reqwest::Url::parse(&provider_config.base_url).map_err(|err| {
                BabataError::config(format!(
                    "Provider '{}' has invalid base_url '{}': {}",
                    provider_name, provider_config.base_url, err
                ))
            })?;

            let scheme = parsed.scheme();
            if scheme != "http" && scheme != "https" {
                return Err(BabataError::config(format!(
                    "Provider '{}' has unsupported base_url scheme '{}', only http/https are allowed",
                    provider_name, scheme
                )));
            }
        }

        let mut has_main_agent = false;
        for (agent_name, agent_config) in &self.agents {
            if agent_name == "main" {
                has_main_agent = true;
            }
            if !self.providers.contains_key(&agent_config.provider) {
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
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_json_roundtrip() {
        let config = Config {
            providers: HashMap::from([(
                "openai".to_string(),
                ProviderConfig {
                    base_url: "https://api.openai.com/v1".to_string(),
                    api_key: "test-api-key".to_string(),
                },
            )]),
            agents: HashMap::from([(
                "main".to_string(),
                AgentConfig {
                    provider: "openai".to_string(),
                    model: "gpt-4.1".to_string(),
                },
            )]),
        };

        let json = serde_json::to_string(&config).expect("serialize config to json");
        let parsed: Config = serde_json::from_str(&json).expect("deserialize config from json");

        assert_eq!(config, parsed);
    }

    #[test]
    fn validate_rejects_invalid_provider_url() {
        let config = Config {
            providers: HashMap::from([(
                "bad-provider".to_string(),
                ProviderConfig {
                    base_url: "not-a-url".to_string(),
                    api_key: "test-api-key".to_string(),
                },
            )]),
            agents: HashMap::from([(
                "main".to_string(),
                AgentConfig {
                    provider: "bad-provider".to_string(),
                    model: "test-model".to_string(),
                },
            )]),
        };

        let err = config
            .validate()
            .expect_err("invalid provider URL should fail");
        let err_msg = err.to_string();
        assert!(err_msg.contains("invalid base_url"));
        assert!(err_msg.contains("bad-provider"));
    }
}
