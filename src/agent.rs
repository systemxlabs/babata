use std::{collections::HashMap, sync::Arc, time::Duration};

use crate::{
    BabataResult,
    channel::{Channel, build_channels},
    config::{AgentConfig, Config},
    error::BabataError,
    memory::Memory,
    message::MessageStore,
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
    pub message_store: MessageStore,
    pub memory: Memory,
    pub tools: HashMap<String, Arc<dyn Tool>>,
    pub system_prompt_files: Vec<SystemPromptFile>,
    pub skills: Vec<Skill>,
}

impl AgentLoop {
    pub fn new(config: Config) -> BabataResult<Self> {
        let providers = build_providers(&config)?;
        let channels = build_channels(&config)?;
        let message_store = MessageStore::new()?;
        let memory = Memory {};
        let tools = build_tools();
        let system_prompt_files = load_system_prompt_files()?;
        let skills = load_skills()?;

        Ok(Self {
            config,
            providers,
            channels,
            message_store,
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
                    self.system_prompt_files.clone(),
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
}
