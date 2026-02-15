use croner::Cron;
use serde::{Deserialize, Serialize};

use crate::{BabataResult, error::BabataError};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct JobConfig {
    pub name: String,
    pub agent_name: String,
    pub enabled: bool,
    pub cron: String,
    #[serde(default)]
    pub description: String,
    pub prompt: String,
}

impl JobConfig {
    pub fn validate(&self) -> BabataResult<()> {
        if self.name.trim().is_empty() {
            return Err(BabataError::config("Job name cannot be empty"));
        }

        if self.agent_name.trim().is_empty() {
            return Err(BabataError::config("Job agent_name cannot be empty"));
        }

        let cron = self.cron.trim();
        if cron.is_empty() {
            return Err(BabataError::config("Job cron expression cannot be empty"));
        }
        Cron::new(cron).parse().map_err(|err| {
            BabataError::config(format!("Invalid cron expression '{}': {}", self.cron, err))
        })?;

        if self.prompt.trim().is_empty() {
            return Err(BabataError::config("Job prompt cannot be empty"));
        }

        Ok(())
    }
}
