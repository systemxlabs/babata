use chrono::{DateTime, Local};
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
        at: DateTime<Local>,
    },
}

impl Schedule {
    pub fn next_run_from_now(&self) -> BabataResult<Option<DateTime<Local>>> {
        let now = Local::now();
        match self {
            Schedule::Cron { expr, .. } => {
                let cron = Cron::new(expr.trim()).parse().map_err(|err| {
                    BabataError::config(format!("Invalid cron expression '{}': {}", expr, err))
                })?;
                let next_run = cron.find_next_occurrence(&now, false).map_err(|err| {
                    BabataError::internal(format!(
                        "Failed to calculate next run time for cron schedule '{}': {}",
                        expr, err
                    ))
                })?;
                Ok(Some(next_run))
            }
            Schedule::At { at } => {
                if now > *at {
                    Ok(None)
                } else {
                    Ok(Some(*at))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;

    #[test]
    fn at_schedule_returns_none_when_time_is_past() {
        let schedule = Schedule::At {
            at: Local::now() - Duration::seconds(1),
        };

        let next_run = schedule
            .next_run_from_now()
            .expect("next_run_from_now should succeed");

        assert!(next_run.is_none());
    }

    #[test]
    fn at_schedule_returns_some_when_time_is_future() {
        let schedule = Schedule::At {
            at: Local::now() + Duration::seconds(5),
        };

        let next_run = schedule
            .next_run_from_now()
            .expect("next_run_from_now should succeed");

        assert!(next_run.is_some());
    }
}
