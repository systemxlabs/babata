use std::sync::Arc;

use crate::{
    channel::Channel,
    config::Config,
    memory::Memory,
    message::MessageStore,
    provider::{Provider, build_providers},
    tool::{BashTool, ReadFileTool, Tool, WriteFileTool},
};

pub struct AgentLoop {
    pub config: Config,
    pub providers: Vec<Arc<dyn Provider>>,
    pub channels: Vec<Arc<dyn Channel>>,
    pub message_store: MessageStore,
    pub memory: Memory,
    pub tools: Vec<Arc<dyn Tool>>,
}

impl AgentLoop {
    pub fn new(config: Config) -> Self {
        let providers = build_providers(&config);
        let channels = Vec::new();
        let message_store = MessageStore::new().expect("Failed to initialize message store");
        let memory = Memory {};
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(BashTool::new()),
            Arc::new(ReadFileTool::new()),
            Arc::new(WriteFileTool::new()),
        ];

        Self {
            config,
            providers,
            channels,
            message_store,
            memory,
            tools,
        }
    }
}
