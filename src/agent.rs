use std::{collections::HashMap, sync::Arc, time::Duration};

use log::{error, info};

use crate::{
    BabataResult,
    channel::{Channel, build_channels},
    config::{AgentConfig, Config},
    error::BabataError,
    memory::Memory,
    message::{Content, Message},
    provider::{Provider, build_providers},
    skill::{Skill, load_skills},
    system_prompt::{SystemPromptFile, load_system_prompt_files},
    task::AgentTask,
    tool::{Tool, build_tools},
};

pub struct AgentLoop {
    pub config: Config,
    pub providers: HashMap<String, Arc<dyn Provider>>,
    pub channels: Vec<Arc<dyn Channel>>,
    pub memory: Memory,
    pub tools: HashMap<String, Arc<dyn Tool>>,
    pub system_prompt_files: Vec<SystemPromptFile>,
    pub skills: Vec<Skill>,
}

impl AgentLoop {
    pub fn new(config: Config) -> BabataResult<Self> {
        let providers = build_providers(&config)?;
        let channels = build_channels(&config)?;
        let memory = Memory::new()?;
        let tools = build_tools();
        let system_prompt_files = load_system_prompt_files()?;
        let skills = load_skills()?;

        Ok(Self {
            config,
            providers,
            channels,
            memory,
            tools,
            system_prompt_files,
            skills,
        })
    }

    pub async fn run(&self) -> BabataResult<()> {
        let agent_config = self.require_agent("main")?;
        let provider = self.require_provider_for_agent(agent_config)?;

        loop {
            let pending_messages = self.collect_messages_from_channels().await;

            if pending_messages.is_empty() {
                tokio::time::sleep(Duration::from_millis(200)).await;
                continue;
            }
            info!("Channel messages: {:?}", pending_messages);

            let context = self.memory.build_context(pending_messages.clone())?;

            self.memory.insert_messages(&pending_messages)?;

            let task = AgentTask::new(
                context,
                Arc::clone(&provider),
                agent_config.model.clone(),
                self.tools.clone(),
                self.system_prompt_files.clone(),
                self.skills.clone(),
            );
            let response = match task.run().await {
                Ok(response) => response,
                Err(err) => {
                    error!("Agent task failed: {}", err);

                    let error_message = agent_task_failed_message(&err);
                    if let Err(send_err) = self.send_to_channels(&error_message).await {
                        error!(
                            "Failed to send agent task error message to channel(s): {}",
                            send_err
                        );
                    }
                    continue;
                }
            };
            info!("Task run result message: {:?}", response);

            self.memory
                .insert_messages(std::slice::from_ref(&response))?;

            self.send_to_channels(&response).await?;
        }
    }

    async fn collect_messages_from_channels(&self) -> Vec<Message> {
        let mut pending_messages = Vec::new();

        for channel in &self.channels {
            let maybe_messages = match channel.try_receive().await {
                Ok(messages) => messages,
                Err(err) => {
                    error!(
                        "Failed to receive messages from channel {:?}: {}. Skipping this channel in current cycle.",
                        channel, err
                    );
                    continue;
                }
            };

            if let Some(messages) = maybe_messages {
                pending_messages.extend(messages);
            }
        }

        pending_messages
    }

    pub(crate) fn require_agent(&self, agent_name: &str) -> BabataResult<&AgentConfig> {
        self.config.get_agent(agent_name).ok_or_else(|| {
            BabataError::config(format!(
                "Agent '{}' not found in config; run onboarding first",
                agent_name
            ))
        })
    }

    pub(crate) fn require_provider_for_agent(
        &self,
        agent_config: &AgentConfig,
    ) -> BabataResult<Arc<dyn Provider>> {
        self.find_provider(&agent_config.provider).ok_or_else(|| {
            BabataError::config(format!(
                "Provider '{}' for agent '{}' not found",
                agent_config.provider, agent_config.name
            ))
        })
    }

    pub(crate) fn find_provider(&self, provider_name: &str) -> Option<Arc<dyn Provider>> {
        self.providers.iter().find_map(|(name, provider)| {
            name.eq_ignore_ascii_case(provider_name)
                .then(|| Arc::clone(provider))
        })
    }

    async fn send_to_channels(&self, message: &Message) -> BabataResult<()> {
        let mut send_failures = Vec::new();
        for channel in &self.channels {
            if let Err(err) = channel.send(std::slice::from_ref(message)).await {
                send_failures.push(err.to_string());
            }
        }

        if send_failures.is_empty() {
            return Ok(());
        }

        Err(BabataError::internal(format!(
            "Failed to send message to {} channel(s): {}",
            send_failures.len(),
            send_failures.join("; ")
        )))
    }
}

fn agent_task_failed_message(err: &BabataError) -> Message {
    Message::AssistantResponse {
        content: vec![Content::Text {
            text: format!("Agent task failed: {}", err),
        }],
        reasoning_content: None,
    }
}
