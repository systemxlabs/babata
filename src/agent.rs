use std::{collections::HashMap, sync::Arc, time::Duration};

use crate::{
    BabataResult,
    channel::{Channel, build_channels},
    config::Config,
    error::BabataError,
    memory::Memory,
    message::MessageStore,
    provider::{Provider, build_providers},
    skill::{Skill, load_skills},
    system_prompt::{SystemPrompt, load_system_prompts},
    task::AgentTask,
    tool::{Tool, build_tools},
};

pub struct AgentLoop {
    pub config: Config,
    pub providers: HashMap<String, Arc<dyn Provider>>,
    pub channels: Vec<Arc<dyn Channel>>,
    pub message_store: MessageStore,
    pub memory: Memory,
    pub tools: HashMap<String, Arc<dyn Tool>>,
    pub system_prompts: Vec<SystemPrompt>,
    pub skills: Vec<Skill>,
}

impl AgentLoop {
    pub fn new(config: Config) -> BabataResult<Self> {
        let providers = build_providers(&config)?;
        let channels = build_channels(&config)?;
        let message_store = MessageStore::new()?;
        let memory = Memory {};
        let tools = build_tools();
        let system_prompts = load_system_prompts()?;
        let skills = load_skills()?;

        Ok(Self {
            config,
            providers,
            channels,
            message_store,
            memory,
            tools,
            system_prompts,
            skills,
        })
    }

    pub async fn run(&self) -> BabataResult<()> {
        if self.channels.is_empty() {
            return Err(BabataError::config(
                "No channels configured; cannot start agent loop",
            ));
        }

        let agent_config = self.config.get_agent("main").ok_or_else(|| {
            BabataError::config("No 'main' agent found in config; run onboarding first")
        })?;

        let provider = self
            .providers
            .iter()
            .find_map(|(provider_name, provider)| {
                provider_name
                    .eq_ignore_ascii_case(&agent_config.provider)
                    .then(|| Arc::clone(provider))
            })
            .ok_or_else(|| {
                BabataError::config(format!(
                    "Provider '{}' for main agent not found",
                    agent_config.provider
                ))
            })?;

        loop {
            let mut handled_message = false;

            for channel in &self.channels {
                let Some(messages) = channel.try_receive().await? else {
                    continue;
                };
                if messages.is_empty() {
                    continue;
                }

                handled_message = true;
                self.message_store.insert_messages(&messages)?;

                let task = AgentTask::new(
                    messages,
                    Arc::clone(&provider),
                    agent_config.model.clone(),
                    self.tools.clone(),
                    self.system_prompts.clone(),
                    self.skills.clone(),
                );
                let response = task.run().await?;

                self.message_store
                    .insert_messages(std::slice::from_ref(&response))?;
                channel.send(std::slice::from_ref(&response)).await?;
            }

            if !handled_message {
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }
    }
}
