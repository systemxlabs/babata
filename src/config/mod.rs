mod channel;
mod job;
mod provider;

pub use channel::*;
pub use job::*;
pub use provider::*;

use std::collections::HashSet;

use crate::{BabataResult, error::BabataError, utils::babata_dir};
use serde::{Deserialize, Serialize};

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
    pub jobs: Vec<JobConfig>,
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

        let mut job_names = HashSet::new();
        for job in &self.jobs {
            job.validate()?;
            if !self.agents.iter().any(|agent| agent.name == job.agent_name) {
                return Err(BabataError::config(format!(
                    "Job '{}' references unknown agent '{}'",
                    job.name, job.agent_name
                )));
            }
            if !job_names.insert(job.name.clone()) {
                return Err(BabataError::config(format!(
                    "Duplicate job name '{}' found in configuration",
                    job.name
                )));
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

    pub fn upsert_job(&mut self, job_config: JobConfig) {
        if let Some(existing) = self
            .jobs
            .iter_mut()
            .find(|existing| existing.name == job_config.name)
        {
            *existing = job_config;
            return;
        }

        self.jobs.push(job_config);
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
            jobs: Vec::new(),
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
            jobs: Vec::new(),
        };

        config.validate().expect("provider URL no longer validated");
    }

    #[test]
    fn validate_rejects_job_with_unknown_agent() {
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
            jobs: vec![JobConfig {
                name: "daily-summary".to_string(),
                agent_name: "non-existent-agent".to_string(),
                enabled: true,
                schedule: Schedule::Cron {
                    expr: "0 9 * * *".to_string(),
                    tz: None,
                },
                description: "Daily summary job".to_string(),
                prompt: "Summarize today's progress".to_string(),
            }],
        };

        assert!(config.validate().is_err());
    }
}
