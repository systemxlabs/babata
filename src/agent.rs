use std::sync::Arc;

use crate::{config::Config, memory::Memory, provider::Provider, tool::Tool};

pub struct AgentLoop {
    pub config: Config,
    pub providers: Vec<Arc<dyn Provider>>,
    pub memory: Arc<dyn Memory>,
    pub tools: Vec<Arc<dyn Tool>>,
}
