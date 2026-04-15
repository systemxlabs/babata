mod config;
mod telegram;
mod wechat;

pub use config::*;
pub use telegram::*;
pub use wechat::*;

use std::collections::HashMap;
use std::time::Duration;
use std::{fmt::Debug, sync::Arc};

use crate::{
    BabataResult,
    message::Content,
    task::{CreateTaskRequest, TaskManager},
};
use log::{error, info, warn};

const CHANNEL_RETRY_DELAY_SECS: u64 = 3;

#[async_trait::async_trait]
pub trait Channel: Debug + Send + Sync {
    // Try to receive messages, returning empty vec if no messages are available
    async fn try_receive(&self) -> BabataResult<Vec<Content>>;

    async fn feedback(&self, content: Vec<Content>) -> BabataResult<Vec<Content>>;
}

pub fn build_channels(
    channel_configs: &[ChannelConfig],
) -> BabataResult<HashMap<String, Arc<dyn Channel>>> {
    let mut channels: HashMap<String, Arc<dyn Channel>> =
        HashMap::with_capacity(channel_configs.len());

    for channel_config in channel_configs {
        match channel_config {
            ChannelConfig::Telegram(telegram_config) => {
                telegram_config.validate()?;
                let channel = TelegramChannel::new(telegram_config.clone())?;
                let channel_name = channel_config.name().to_string();
                if channels.contains_key(&channel_name) {
                    return Err(crate::error::BabataError::config(format!(
                        "Duplicate channel name '{}' found in channel configs",
                        channel_name
                    )));
                }
                channels.insert(channel_name, Arc::new(channel));
            }
            ChannelConfig::Wechat(wechat_config) => {
                wechat_config.validate()?;
                let channel = WechatChannel::new(wechat_config.clone())?;
                let channel_name = channel_config.name().to_string();
                if channels.contains_key(&channel_name) {
                    return Err(crate::error::BabataError::config(format!(
                        "Duplicate channel name '{}' found in channel configs",
                        channel_name
                    )));
                }
                channels.insert(channel_name, Arc::new(channel));
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
                        if !content.is_empty() {
                            let description = render_prompt_as_description(&content);

                            let mut prompt = vec![Content::Text {
                                text: format!(
                                    "Your user sent you a message from {}, the message content is below:",
                                    channel_name
                                ),
                            }];
                            prompt.extend(content);

                            let task = CreateTaskRequest {
                                description,
                                prompt,
                                parent_task_id: None,
                                agent: task_manager.default_agent().frontmatter.name.clone(),
                                never_ends: false,
                            };

                            if let Err(err) = task_manager.create_task(task) {
                                error!(
                                    "Failed to create task from channel '{}': {}",
                                    channel_name, err
                                );
                            }
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

fn render_prompt_as_description(prompt: &[Content]) -> String {
    let lines = prompt
        .iter()
        .map(|content| match content {
            Content::Text { text } => text.clone(),
            Content::ImageUrl { url } => format!("- [image] {url}"),
            Content::ImageData { media_type, .. } => format!("- [image_data] {media_type}"),
            Content::AudioData { media_type, .. } => format!("- [audio_data] {media_type}"),
        })
        .collect::<Vec<_>>();

    if lines.is_empty() {
        "_No prompt provided._".to_string()
    } else {
        lines.join("\n\n")
    }
}
