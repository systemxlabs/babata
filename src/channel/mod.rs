mod telegram;

pub use telegram::*;

use std::collections::HashMap;
use std::time::Duration;
use std::{fmt::Debug, sync::Arc};

use crate::{
    BabataResult,
    config::{ChannelConfig, Config},
    message::Content,
    task::{TaskManager, TaskRequest},
};
use log::{error, info, warn};

const CHANNEL_RETRY_DELAY_SECS: u64 = 3;

#[async_trait::async_trait]
pub trait Channel: Debug + Send + Sync {
    // Channel name, e.g. "telegram"
    fn name() -> &'static str
    where
        Self: Sized;

    // Try to receive messages, returning empty vec if no messages are available
    async fn try_receive(&self) -> BabataResult<Vec<Content>>;

    async fn feedback(&self, content: Vec<Content>) -> BabataResult<Vec<Content>>;
}

pub fn build_channels(config: &Config) -> BabataResult<HashMap<String, Arc<dyn Channel>>> {
    let mut channels: HashMap<String, Arc<dyn Channel>> =
        HashMap::with_capacity(config.channels.len());

    for channel_config in &config.channels {
        match channel_config {
            ChannelConfig::Telegram(telegram_config) => {
                telegram_config.validate()?;
                let channel = TelegramChannel::new(telegram_config.clone());
                channels.insert(
                    channel_config.name().to_ascii_lowercase(),
                    Arc::new(channel),
                );
            }
        }
    }

    Ok(channels)
}

pub fn start_channel_loops(
    channels: HashMap<String, Arc<dyn Channel>>,
    task_manager: Arc<TaskManager>,
) {
    for (channel_name, channel) in channels {
        let task_manager = task_manager.clone();
        tokio::spawn(async move {
            info!("Starting channel loop '{}'", channel_name);
            loop {
                match channel.try_receive().await {
                    Ok(content) => {
                        if !content.is_empty()
                            && let Err(err) = task_manager.create_task(TaskRequest {
                                prompt: content,
                                parent_task_id: None,
                                agent: None,
                            })
                        {
                            error!(
                                "Failed to create task from channel '{}': {}",
                                channel_name, err
                            );
                        }
                    }
                    Err(err) => {
                        warn!("Channel '{}' receive failed: {}", channel_name, err);
                    }
                }

                tokio::time::sleep(Duration::from_secs(CHANNEL_RETRY_DELAY_SECS)).await;
            }
        });
    }
}
