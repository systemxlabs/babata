use std::sync::Arc;

use crate::{
    channel::Channel, config::Config, memory::Memory, provider::Provider, message::MessageStore,
    tool::Tool,
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
    pub fn new(_config: Config) -> Self {
        todo!()
    }
}
