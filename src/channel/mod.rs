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
                let channel = TelegramChannel::new(telegram_config.clone());
                channels.push(Arc::new(channel));
            }
        }
    }

    Ok(channels)
}
