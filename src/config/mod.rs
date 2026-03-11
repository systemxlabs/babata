mod agent;
mod channel;
mod memory;
mod provider;

pub use agent::*;
pub use channel::*;
pub use memory::*;
pub use provider::*;

use std::collections::HashSet;

use crate::{
    BabataResult,
    agent::{Agent, babata::BabataAgent},
    error::BabataError,
    memory::{Memory, SimpleMemory},
    utils::babata_dir,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Config {
    pub providers: Vec<ProviderConfig>,
    pub agents: Vec<AgentConfig>,
    #[serde(default)]
    pub channels: Vec<ChannelConfig>,
    #[serde(default)]
    pub memory: Vec<MemoryConfig>,
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
                memory: Vec::new(),
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
        if let Some(existing) = self.agents.iter_mut().find(|existing| {
            matches!(
                (existing, &agent_config),
                (AgentConfig::Babata(_), AgentConfig::Babata(_))
            )
        }) {
            *existing = agent_config;
            return;
        }

        self.agents.push(agent_config);
    }

    pub fn upsert_memory(&mut self, memory_config: MemoryConfig) {
        if let Some(existing) = self.memory.iter_mut().find(|existing| {
            matches!(
                (&**existing, &memory_config),
                (MemoryConfig::Simple, MemoryConfig::Simple)
                    | (MemoryConfig::Hybrid(_), MemoryConfig::Hybrid(_))
            )
        }) {
            *existing = memory_config;
            return;
        }

        self.memory.push(memory_config);
    }

    pub fn get_memory(&self, memory_name: &str) -> Option<&MemoryConfig> {
        self.memory.iter().find(|memory| match memory {
            MemoryConfig::Simple => memory_name.eq_ignore_ascii_case(SimpleMemory::name()),
            MemoryConfig::Hybrid(_) => memory_name.eq_ignore_ascii_case("hybrid"),
        })
    }

    pub fn get_agent(&self, agent_name: &str) -> BabataResult<&AgentConfig> {
        self.agents
            .iter()
            .find(|agent| match agent {
                AgentConfig::Babata(_) => agent_name.eq_ignore_ascii_case(BabataAgent::name()),
            })
            .ok_or_else(|| {
                BabataError::config(format!("Agent '{}' not found in config", agent_name))
            })
    }

    pub fn get_provider(&self, provider_name: &str) -> BabataResult<&ProviderConfig> {
        self.providers
            .iter()
            .find(|provider| provider.matches_name(provider_name))
            .ok_or_else(|| {
                BabataError::config(format!("Provider '{}' not found in config", provider_name))
            })
    }
}
