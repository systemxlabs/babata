use std::{collections::HashMap, sync::Arc};

use crate::{
    channel::Channel,
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
    pub providers: Vec<Arc<dyn Provider>>,
    pub channels: Vec<Arc<dyn Channel>>,
    pub message_store: MessageStore,
    pub memory: Memory,
    pub tools: HashMap<String, Arc<dyn Tool>>,
    pub system_prompts: Vec<SystemPrompt>,
    pub skills: Vec<Skill>,
}

impl AgentLoop {
    pub fn new(config: Config) -> Self {
        let providers = build_providers(&config);
        let channels = Vec::new();
        let message_store = MessageStore::new().expect("Failed to initialize message store");
        let memory = Memory {};
        let tools = build_tools();
        let system_prompts = load_system_prompts().unwrap_or_else(|err| {
            panic!("Failed to load system prompts: {err}");
        });
        let skills = load_skills().unwrap_or_else(|err| {
            panic!("Failed to load skills: {err}");
        });

        Self {
            config,
            providers,
            channels,
            message_store,
            memory,
            tools,
            system_prompts,
            skills,
        }
    }
}
