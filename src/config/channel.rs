use serde::{Deserialize, Serialize};

use crate::{
    BabataResult,
    channel::{Channel, TelegramChannel, WechatChannel},
    error::BabataError,
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "name", rename_all = "snake_case")]
pub enum ChannelConfig {
    Telegram(TelegramChannelConfig),
    Wechat(WechatChannelConfig),
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TelegramChannelConfig {
    pub bot_token: String,
    pub user_id: i64,
}

impl TelegramChannelConfig {
    pub fn validate(&self) -> BabataResult<()> {
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
    pub token: String,
    pub user_id: String,
}

impl WechatChannelConfig {
    pub fn validate(&self) -> BabataResult<()> {
        if self.token.trim().is_empty() {
            return Err(BabataError::config("Wechat channel token cannot be empty"));
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
    pub fn name(&self) -> &'static str {
        match self {
            ChannelConfig::Telegram(_) => TelegramChannel::name(),
            ChannelConfig::Wechat(_) => WechatChannel::name(),
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
    fn telegram_config_rejects_empty_bot_token() {
        let config = TelegramChannelConfig {
            bot_token: "   ".to_string(),
            user_id: 12345,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn telegram_config_rejects_non_positive_user_id() {
        let config = TelegramChannelConfig {
            bot_token: "token".to_string(),
            user_id: 0,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn wechat_config_rejects_empty_token() {
        let config = WechatChannelConfig {
            token: " ".to_string(),
            user_id: "wxid_123".to_string(),
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn wechat_config_rejects_empty_user_id() {
        let config = WechatChannelConfig {
            token: "token".to_string(),
            user_id: " ".to_string(),
        };

        assert!(config.validate().is_err());
    }
}
