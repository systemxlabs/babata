mod channel;

pub use channel::*;

use crate::{BabataResult, error::BabataError, utils::babata_dir};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub channels: Vec<ChannelConfig>,
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
            Ok(Self::default())
        }
    }

    pub fn load() -> BabataResult<Self> {
        let config_path = Self::path()?;
        Self::load_from_path(&config_path)
    }

    fn load_from_path(config_path: &std::path::Path) -> BabataResult<Self> {
        let raw = std::fs::read_to_string(config_path).map_err(|err| {
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
        self.save_to_path(&config_path)
    }

    fn save_to_path(&self, config_path: &std::path::Path) -> BabataResult<()> {
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

        std::fs::write(config_path, payload).map_err(|err| {
            BabataError::config(format!(
                "Failed to write config file '{}': {}",
                config_path.display(),
                err
            ))
        })?;

        Ok(())
    }

    pub fn validate(&self) -> BabataResult<()> {
        for channel in &self.channels {
            channel.validate()?;
        }

        Ok(())
    }

    pub fn upsert_channel(&mut self, channel_config: ChannelConfig) {
        if let Some(existing) = self.channels.iter_mut().find(|existing| {
            matches!(
                (existing, &channel_config),
                (ChannelConfig::Telegram(_), ChannelConfig::Telegram(_))
                    | (ChannelConfig::Wechat(_), ChannelConfig::Wechat(_))
            )
        }) {
            *existing = channel_config;
            return;
        }

        self.channels.push(channel_config);
    }
}
