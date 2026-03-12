mod telegram;

pub use telegram::*;

use std::{fmt::Debug, sync::Arc};

use crate::{
    BabataResult,
    config::{ChannelConfig, Config},
    message::{Content, Message},
    task::TaskManager,
};

#[async_trait::async_trait]
pub trait Channel: Debug + Send + Sync {
    // Channel name, e.g. "telegram"
    fn name() -> &'static str
    where
        Self: Sized;

    // Try to receive messages, returning None if no messages are available
    async fn start(&self, task_manager: Arc<TaskManager>) -> BabataResult<()>;

    async fn feedback(&self, content: Vec<Content>) -> BabataResult<Vec<Content>>;
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
