use chrono::{DateTime, Utc};
use croner::Cron;
use serde::{Deserialize, Serialize};

use crate::{BabataResult, error::BabataError};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct JobConfig {
    pub name: String,
    pub agent_name: String,
    pub enabled: bool,
    pub schedule: Schedule,
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

        match &self.schedule {
            Schedule::Cron { expr, .. } => {
                let cron = expr.trim();
                if cron.is_empty() {
                    return Err(BabataError::config(
                        "Job schedule.cron expression cannot be empty",
                    ));
                }
                Cron::new(cron).parse().map_err(|err| {
                    BabataError::config(format!("Invalid cron expression '{}': {}", expr, err))
                })?;
            }
            Schedule::At { .. } => {}
        }

        if self.prompt.trim().is_empty() {
            return Err(BabataError::config("Job prompt cannot be empty"));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Schedule {
    Cron {
        expr: String,
        #[serde(default)]
        tz: Option<String>,
    },
    At {
        at: DateTime<Utc>,
    },
}
