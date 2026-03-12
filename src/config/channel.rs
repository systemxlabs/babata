use serde::{Deserialize, Serialize};

use crate::{
    BabataResult,
    channel::{Channel, TelegramChannel},
    error::BabataError,
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "name", rename_all = "snake_case")]
pub enum ChannelConfig {
    Telegram(TelegramChannelConfig),
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TelegramChannelConfig {
    pub bot_token: String,
    #[serde(default)]
    pub last_update_id: Option<i64>,
    pub user_id: i64,
}

impl TelegramChannelConfig {
    pub fn last_update_id(&self) -> Option<i64> {
        self.last_update_id
    }

    pub fn validate(&self) -> BabataResult<()> {
        if self.bot_token.trim().is_empty() {
            return Err(BabataError::config(
                "Telegram channel bot_token cannot be empty",
            ));
        }

        if let Some(last_update_id) = self.last_update_id
            && last_update_id < 0
        {
            return Err(BabataError::config(
                "Telegram channel last_update_id must be greater than or equal to 0",
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

impl ChannelConfig {
    pub fn name(&self) -> &'static str {
        match self {
            ChannelConfig::Telegram(_) => TelegramChannel::name(),
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
            last_update_id: None,
            user_id: 12345,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn telegram_config_rejects_non_positive_user_id() {
        let config = TelegramChannelConfig {
            bot_token: "token".to_string(),
            last_update_id: None,
            user_id: 0,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn telegram_config_rejects_negative_last_update_id() {
        let config = TelegramChannelConfig {
            bot_token: "token".to_string(),
            last_update_id: Some(-1),
            user_id: 12345,
        };

        assert!(config.validate().is_err());
    }
}
