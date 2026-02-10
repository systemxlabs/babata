use std::collections::HashMap;

use crate::{BabataResult, error::BabataError};

pub struct Config {
    pub default_system_prompt: String,
    pub default_skills: Vec<SkillConfig>,
    pub providers: HashMap<String, ProviderConfig>,
    pub agents: HashMap<String, AgentConfig>,
}

impl Config {
    pub fn validate(&self) -> BabataResult<()> {
        for skill in &self.default_skills {
            if std::fs::exists(&skill.path)? {
                return Err(BabataError::config(format!(
                    "Default skill path '{}' does not exist",
                    skill.path
                )));
            }
        }

        for (agent_name, agent_config) in &self.agents {
            if !self.providers.contains_key(&agent_config.provider) {
                return Err(BabataError::config(format!(
                    "Agent '{}' references unknown provider '{}'",
                    agent_name, agent_config.provider
                )));
            }
        }
        Ok(())
    }
}

pub struct AgentConfig {
    // If None, use default system prompt
    pub system_prompt: Option<String>,
    // If None, use default skills
    pub skills: Option<Vec<SkillConfig>>,
    pub provider: String,
    pub model: String,
}

pub struct ProviderConfig {
    // The completed URL for the provider's API
    pub base_url: String,
    // The API key for authentication
    pub api_key: String,
}

pub struct SkillConfig {
    // Whether the skill is enabled
    pub enabled: bool,
    // Whether the whole skill.md is inlined in prompt
    pub inlined: bool,
    // Absolute path to the skill dir
    pub path: String,
}
