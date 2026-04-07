use serde::{Deserialize, Serialize};

use crate::{
    BabataResult,
    agent::{Agent, babata::BabataAgent},
    error::BabataError,
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

    pub fn validate(&self) -> BabataResult<()> {
        match self {
            AgentConfig::Babata(config) => config.validate(),
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

impl BabataAgentConfig {
    pub fn validate(&self) -> BabataResult<()> {
        if self.provider.trim().is_empty() {
            return Err(BabataError::config(
                "Babata agent provider must not be empty",
            ));
        }
        if self.model.trim().is_empty() {
            return Err(BabataError::config("Babata agent model must not be empty"));
        }
        if self.memory.trim().is_empty() {
            return Err(BabataError::config("Babata agent memory must not be empty"));
        }
        Ok(())
    }
}
