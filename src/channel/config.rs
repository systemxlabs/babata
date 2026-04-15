use std::{collections::HashSet, path::Path};

use serde::{Deserialize, Serialize};

use crate::{
    BabataResult,
    error::BabataError,
    utils::{channel_dir, channels_dir},
};

const CHANNEL_CONFIG_FILE_NAME: &str = "config.json";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChannelConfig {
    Telegram(TelegramChannelConfig),
    Wechat(WechatChannelConfig),
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TelegramChannelConfig {
    pub name: String,
    pub bot_token: String,
    pub user_id: i64,
}

impl TelegramChannelConfig {
    pub fn validate(&self) -> BabataResult<()> {
        validate_channel_name(&self.name)?;

        if self.bot_token.trim().is_empty() {
            return Err(BabataError::config(
                "Telegram channel bot_token cannot be empty",
            ));
        }

        if self.user_id <= 0 {
            return Err(BabataError::config(
                "Telegram channel user_id must be a positive value",
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct WechatChannelConfig {
    pub name: String,
    pub bot_token: String,
    pub user_id: String,
}

impl WechatChannelConfig {
    pub fn validate(&self) -> BabataResult<()> {
        validate_channel_name(&self.name)?;

        if self.bot_token.trim().is_empty() {
            return Err(BabataError::config(
                "Wechat channel bot_token cannot be empty",
            ));
        }

        if self.user_id.trim().is_empty() {
            return Err(BabataError::config(
                "Wechat channel user_id cannot be empty",
            ));
        }

        Ok(())
    }
}

impl ChannelConfig {
    pub fn name(&self) -> &str {
        match self {
            ChannelConfig::Telegram(config) => &config.name,
            ChannelConfig::Wechat(config) => &config.name,
        }
    }

    pub fn validate(&self) -> BabataResult<()> {
        match self {
            ChannelConfig::Telegram(config) => config.validate(),
            ChannelConfig::Wechat(config) => config.validate(),
        }
    }

    pub fn matches_name(&self, name: &str) -> bool {
        self.name().eq_ignore_ascii_case(name)
    }

    pub fn load(name: &str) -> BabataResult<Self> {
        validate_channel_name(name)?;
        let config_path = channel_dir(name)?.join(CHANNEL_CONFIG_FILE_NAME);
        let channel_config = load_from_path(&config_path)?;
        if !channel_config.matches_name(name) {
            return Err(BabataError::config(format!(
                "Channel config file '{}' does not match directory name '{}'",
                config_path.display(),
                name
            )));
        }
        Ok(channel_config)
    }

    pub fn load_all() -> BabataResult<Vec<Self>> {
        let channels_dir = channels_dir()?;
        load_all_from_dir(&channels_dir)
    }

    pub fn save(&self) -> BabataResult<()> {
        self.validate()?;

        let channel_dir = channel_dir(self.name())?;
        std::fs::create_dir_all(&channel_dir)?;

        let payload = serde_json::to_string_pretty(self)
            .map_err(|err| BabataError::config(format!("Failed to serialize channel: {}", err)))?;
        let config_path = channel_dir.join(CHANNEL_CONFIG_FILE_NAME);
        std::fs::write(&config_path, payload)?;

        Ok(())
    }

    pub fn delete(name: &str) -> BabataResult<()> {
        validate_channel_name(name)?;
        let channel_dir = channel_dir(name)?;
        if !channel_dir.exists() {
            return Err(BabataError::not_found(format!(
                "Channel '{}' not found",
                name
            )));
        }

        std::fs::remove_dir_all(&channel_dir)?;

        Ok(())
    }
}

fn load_all_from_dir(channels_dir: &Path) -> BabataResult<Vec<ChannelConfig>> {
    if !channels_dir.exists() {
        return Ok(Vec::new());
    }

    if !channels_dir.is_dir() {
        return Err(BabataError::config(format!(
            "Channels path '{}' is not a directory",
            channels_dir.display()
        )));
    }

    let entries = std::fs::read_dir(channels_dir)?;

    let mut channels = Vec::new();
    let mut channel_names = HashSet::new();
    for entry in entries {
        let entry = entry.map_err(|err| {
            BabataError::config(format!(
                "Failed to read channels directory entry in '{}': {}",
                channels_dir.display(),
                err
            ))
        })?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let Some(channel_name) = path.file_name().and_then(|name| name.to_str()) else {
            return Err(BabataError::config(format!(
                "Channel directory '{}' is not valid UTF-8",
                path.display()
            )));
        };

        let channel = ChannelConfig::load(channel_name)?;
        let normalized_name = channel.name().to_ascii_lowercase();
        if !channel_names.insert(normalized_name) {
            return Err(BabataError::config(format!(
                "Duplicate channel name '{}' found in channels directory",
                channel.name()
            )));
        }
        channels.push(channel);
    }

    channels.sort_by_cached_key(|channel| channel.name().to_ascii_lowercase());
    Ok(channels)
}

fn load_from_path(config_path: &Path) -> BabataResult<ChannelConfig> {
    let raw = std::fs::read_to_string(config_path)?;
    let channel_config = serde_json::from_str::<ChannelConfig>(&raw).map_err(|err| {
        BabataError::config(format!(
            "Failed to parse channel config file '{}': {}",
            config_path.display(),
            err
        ))
    })?;
    channel_config.validate()?;
    Ok(channel_config)
}

fn validate_channel_name(name: &str) -> BabataResult<()> {
    if name.is_empty() {
        return Err(BabataError::config("Channel name cannot be empty"));
    }
    if name.trim() != name {
        return Err(BabataError::config(
            "Channel name cannot have leading or trailing whitespace",
        ));
    }
    if name == "." || name == ".." || name.contains('/') || name.contains('\\') {
        return Err(BabataError::config(
            "Channel name cannot contain path separators or reserved relative segments",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telegram_config_rejects_empty_bot_token() {
        let config = TelegramChannelConfig {
            name: "telegram-main".to_string(),
            bot_token: "   ".to_string(),
            user_id: 12345,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn telegram_config_rejects_non_positive_user_id() {
        let config = TelegramChannelConfig {
            name: "telegram-main".to_string(),
            bot_token: "token".to_string(),
            user_id: 0,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn wechat_config_rejects_empty_bot_token() {
        let config = WechatChannelConfig {
            name: "wechat-main".to_string(),
            bot_token: " ".to_string(),
            user_id: "wxid_123".to_string(),
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn wechat_config_rejects_empty_user_id() {
        let config = WechatChannelConfig {
            name: "wechat-main".to_string(),
            bot_token: "token".to_string(),
            user_id: " ".to_string(),
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn parse_channel_config_from_json() {
        let payload = r#"{
            "kind": "telegram",
            "name": "telegram-main",
            "bot_token": "test-token",
            "user_id": 123456
        }"#;

        let parsed: ChannelConfig = serde_json::from_str(payload).expect("parse channel json");

        assert_eq!(
            parsed,
            ChannelConfig::Telegram(TelegramChannelConfig {
                name: "telegram-main".to_string(),
                bot_token: "test-token".to_string(),
                user_id: 123456,
            })
        );
    }
}
