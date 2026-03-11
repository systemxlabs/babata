mod telegram;

pub use telegram::*;

use std::{fmt::Debug, sync::Arc};

use crate::{
    BabataResult,
    config::{ChannelConfig, Config},
    message::Message,
};

#[async_trait::async_trait]
pub trait Channel: Debug + Send + Sync {
    // Channel name, e.g. "telegram"
    fn name() -> &'static str
    where
        Self: Sized;

    // Receive messages, blocking until messages are available
    async fn receive(&self) -> BabataResult<Vec<Message>>;
    // Try to receive messages, returning None if no messages are available
    async fn try_receive(&self) -> BabataResult<Option<Vec<Message>>>;
}

pub fn build_channels(config: &Config) -> BabataResult<Vec<Arc<dyn Channel>>> {
    let mut channels: Vec<Arc<dyn Channel>> = Vec::with_capacity(config.channels.len());

    for channel_config in &config.channels {
        match channel_config {
            ChannelConfig::Telegram(telegram_config) => {
                telegram_config.validate()?;
                let channel = TelegramChannel::new(&telegram_config.bot_token)
                    .with_polling_timeout_secs(telegram_config.polling_timeout_secs())
                    .with_last_update_id(telegram_config.last_update_id())
                    .with_allowed_user_ids(telegram_config.allowed_user_ids.clone());
                channels.push(Arc::new(channel));
            }
        }
    }

    Ok(channels)
}
