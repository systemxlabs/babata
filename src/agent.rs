use std::{collections::HashMap, sync::Arc};

use crate::{
    BabataResult,
    channel::{Channel, build_channels},
    config::Config,
    memory::Memory,
    message::MessageStore,
    provider::{Provider, build_providers},
    skill::{Skill, load_skills},
    system_prompt::{SystemPrompt, load_system_prompts},
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
}
