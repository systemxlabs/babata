use serde::{Deserialize, Serialize};

use crate::{
    BabataResult,
    agent::{Agent, babata::BabataAgent, codex::CodexAgent, opencode::OpencodeAgent},
    error::BabataError,
    memory::{Memory, SimpleMemory},
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "name", rename_all = "snake_case")]
pub enum AgentConfig {
    Babata(BabataAgentConfig),
    Codex(CodexAgentConfig),
    Opencode(OpencodeAgentConfig),
}

impl AgentConfig {
    pub fn name(&self) -> &'static str {
        match self {
            AgentConfig::Babata(_) => BabataAgent::name(),
            AgentConfig::Codex(_) => CodexAgent::name(),
            AgentConfig::Opencode(_) => OpencodeAgent::name(),
        }
    }

    pub fn validate(&self) -> BabataResult<()> {
        match self {
            AgentConfig::Babata(config) => config.validate(),
            AgentConfig::Codex(config) => config.validate(),
            AgentConfig::Opencode(config) => config.validate(),
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct CodexAgentConfig {
    pub command: String,
    pub workspace: String,
    #[serde(default)]
    pub model: Option<String>,
}

impl CodexAgentConfig {
    pub fn validate(&self) -> BabataResult<()> {
        if self.command.trim().is_empty() {
            return Err(BabataError::config("Codex agent command must not be empty"));
        }
        if self.workspace.trim().is_empty() {
            return Err(BabataError::config(
                "Codex agent workspace must not be empty",
            ));
        }

        let workspace = std::path::Path::new(&self.workspace);
        if !workspace.exists() {
            return Err(BabataError::config(format!(
                "Codex agent workspace '{}' does not exist",
                workspace.display()
            )));
        }
        if !workspace.is_dir() {
            return Err(BabataError::config(format!(
                "Codex agent workspace '{}' is not a directory",
                workspace.display()
            )));
        }
        if matches!(self.model.as_deref(), Some(model) if model.trim().is_empty()) {
            return Err(BabataError::config(
                "Codex agent model must not be empty when provided",
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct OpencodeAgentConfig {
    pub command: String,
    pub workspace: String,
    #[serde(default)]
    pub model: Option<String>,
}

impl OpencodeAgentConfig {
    pub fn validate(&self) -> BabataResult<()> {
        if self.command.trim().is_empty() {
            return Err(BabataError::config(
                "Opencode agent command must not be empty",
            ));
        }
        if self.workspace.trim().is_empty() {
            return Err(BabataError::config(
                "Opencode agent workspace must not be empty",
            ));
        }

        let workspace = std::path::Path::new(&self.workspace);
        if !workspace.exists() {
            return Err(BabataError::config(format!(
                "Opencode agent workspace '{}' does not exist",
                workspace.display()
            )));
        }
        if !workspace.is_dir() {
            return Err(BabataError::config(format!(
                "Opencode agent workspace '{}' is not a directory",
                workspace.display()
            )));
        }
        if matches!(self.model.as_deref(), Some(model) if model.trim().is_empty()) {
            return Err(BabataError::config(
                "Opencode agent model must not be empty when provided",
            ));
        }

        Ok(())
    }
}
