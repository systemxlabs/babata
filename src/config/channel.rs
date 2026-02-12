use serde::{Deserialize, Serialize};

use crate::{BabataResult, error::BabataError};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChannelConfig {
    Telegram(TelegramChannelConfig),
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct TelegramChannelConfig {
    pub bot_token: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub polling_timeout_secs: Option<u64>,
}

impl TelegramChannelConfig {
    pub const DEFAULT_BASE_URL: &'static str = "https://api.telegram.org";
    pub const DEFAULT_POLLING_TIMEOUT_SECS: u64 = 30;

    pub fn base_url(&self) -> &str {
        self.base_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(Self::DEFAULT_BASE_URL)
    }

    pub fn polling_timeout_secs(&self) -> u64 {
        self.polling_timeout_secs
            .unwrap_or(Self::DEFAULT_POLLING_TIMEOUT_SECS)
    }

    pub fn validate(&self) -> BabataResult<()> {
        if self.bot_token.trim().is_empty() {
            return Err(BabataError::config(
                "Telegram channel bot_token cannot be empty",
            ));
        }

        if let Some(base_url) = &self.base_url
            && base_url.trim().is_empty()
        {
            return Err(BabataError::config(
                "Telegram channel base_url cannot be empty when provided",
            ));
        }

        if self.polling_timeout_secs() == 0 {
            return Err(BabataError::config(
                "Telegram channel polling_timeout_secs must be greater than 0",
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telegram_config_defaults_work() {
        let config = TelegramChannelConfig {
            bot_token: "token".to_string(),
            base_url: None,
            polling_timeout_secs: None,
        };

        assert_eq!(config.base_url(), TelegramChannelConfig::DEFAULT_BASE_URL);
        assert_eq!(
            config.polling_timeout_secs(),
            TelegramChannelConfig::DEFAULT_POLLING_TIMEOUT_SECS
        );
    }

    #[test]
    fn telegram_config_rejects_empty_bot_token() {
        let config = TelegramChannelConfig {
            bot_token: "   ".to_string(),
            base_url: None,
            polling_timeout_secs: None,
        };

        assert!(config.validate().is_err());
    }
}
