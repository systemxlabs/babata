use serde::{Deserialize, Serialize};

use crate::{
    agent::{Agent, babata::BabataAgent},
    memory::{Memory, SimpleMemory},
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "name", rename_all = "snake_case")]
pub enum AgentConfig {
    Babata(BabataAgentConfig),
}

impl AgentConfig {
    pub fn name(&self) -> &'static str {
        match self {
            AgentConfig::Babata(_) => BabataAgent::name(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct BabataAgentConfig {
    pub provider: String,
    pub model: String,
    #[serde(default = "default_memory")]
    pub memory: String,
}

fn default_memory() -> String {
    SimpleMemory::name().to_string()
}
