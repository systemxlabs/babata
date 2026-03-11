use std::sync::Arc;

use log::{error, warn};

use crate::{
    BabataResult,
    channel::{Channel, build_channels},
    config::Config,
    message::{Content, Message},
    runtime::TaskRuntime,
};

#[derive(Debug)]
pub struct ServerApp {
    pub runtime: Arc<TaskRuntime>,
    pub channels: Vec<Arc<dyn Channel>>,
    pub agent_name: String,
}

impl ServerApp {
    pub fn new(config: Config) -> BabataResult<Self> {
        let channels = build_channels(&config)?;
        let runtime = Arc::new(TaskRuntime::new(config)?);
        Ok(Self {
            runtime,
            channels,
            agent_name: "main".to_string(),
        })
    }

    pub async fn run(&self) -> BabataResult<()> {
        self.runtime.resume_running_tasks().await?;

        for channel in &self.channels {
            let channel = Arc::clone(channel);
            let runtime = Arc::clone(&self.runtime);
            let agent_name = self.agent_name.clone();

            tokio::spawn(async move {
                loop {
                    match channel.receive().await {
                        Ok(messages) => {
                            for message in messages {
                                let runtime = Arc::clone(&runtime);
                                let channel = Arc::clone(&channel);
                                let agent_name = agent_name.clone();

                                tokio::spawn(async move {
                                    if let Err(err) = process_channel_message(
                                        runtime,
                                        channel,
                                        &agent_name,
                                        message,
                                    )
                                    .await
                                    {
                                        error!("Channel task processing failed: {}", err);
                                    }
                                });
                            }
                        }
                        Err(err) => {
                            warn!("Channel receive failed: {}. Retrying.", err);
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        }
                    }
                }
            });
        }

        std::future::pending::<()>().await;
        #[allow(unreachable_code)]
        Ok(())
    }
}

async fn process_channel_message(
    runtime: Arc<TaskRuntime>,
    channel: Arc<dyn Channel>,
    agent_name: &str,
    message: Message,
) -> BabataResult<()> {
    let task_id = runtime.submit_prompt_task(agent_name, message).await?;
    match runtime.wait_for_task(&task_id).await {
        Ok(response) => channel.send(std::slice::from_ref(&response)).await,
        Err(err) => {
            let response = task_failed_message(&err);
            channel.send(std::slice::from_ref(&response)).await
        }
    }
}

pub fn task_failed_message(err: &crate::error::BabataError) -> Message {
    Message::AssistantResponse {
        content: vec![Content::Text {
            text: format!("Task failed: {}", err),
        }],
        reasoning_content: None,
    }
}
