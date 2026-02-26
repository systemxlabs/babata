use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use log::error;

use crate::{
    BabataResult,
    channel::build_channels,
    config::{AgentConfig, Config, JobConfig, ProviderConfig},
    error::BabataError,
    message::{Content, Message},
    provider::create_provider,
    skill::load_skills,
    system_prompt::load_system_prompt_files,
    task::AgentTask,
    tool::build_tools,
};

use super::history::{JobHistoryEntry, JobHistoryStore, JobRunStatus};

pub struct JobRunner {
    pub config: Config,
    pub job_name: String,
    pub history_store: JobHistoryStore,
}

impl JobRunner {
    pub fn new(config: Config, job_name: String, history_store: JobHistoryStore) -> Self {
        Self {
            config,
            job_name,
            history_store,
        }
    }

    pub async fn run(&self) -> BabataResult<()> {
        let job_config = self.require_job()?.clone();
        let job_config_payload = serde_json::to_string(&job_config).map_err(|err| {
            BabataError::internal(format!(
                "Failed to serialize job config '{}' into JSON: {}",
                job_config.name, err
            ))
        })?;
        let started_at_epoch = current_epoch_seconds();
        let started_at = Instant::now();

        let run_result = self.run_task_and_send(&job_config).await;
        let finished_at_epoch = current_epoch_seconds();
        let duration_ms = elapsed_millis_i64(started_at.elapsed());

        match &run_result {
            Ok(response) => {
                self.persist_job_history(&JobHistoryEntry {
                    job_name: self.job_name.clone(),
                    job_config: job_config_payload,
                    status: JobRunStatus::Success,
                    response: serde_json::to_string(response).ok(),
                    error: None,
                    started_at_epoch,
                    finished_at_epoch,
                    duration_ms,
                });
            }
            Err(err) => {
                self.persist_job_history(&JobHistoryEntry {
                    job_name: self.job_name.clone(),
                    job_config: job_config_payload,
                    status: JobRunStatus::Failed,
                    response: None,
                    error: Some(err.to_string()),
                    started_at_epoch,
                    finished_at_epoch,
                    duration_ms,
                });
            }
        }

        run_result.map(|_| ())
    }

    async fn run_task_and_send(&self, job_config: &JobConfig) -> BabataResult<Message> {
        let agent_config = self.require_agent(&job_config.agent_name)?;
        let provider_config = self.require_provider_config_for_agent(agent_config)?;
        let provider = create_provider(provider_config)?;

        let task = AgentTask::new(
            vec![Message::UserPrompt {
                content: vec![Content::Text {
                    text: job_config.prompt.clone(),
                }],
            }],
            provider,
            agent_config.model.clone(),
            build_tools(),
            load_system_prompt_files()?,
            load_skills()?,
        );

        let response = task.run().await?;
        let channels = build_channels(&self.config)?;
        let mut send_failures = Vec::new();

        for channel in channels {
            if let Err(err) = channel.send(std::slice::from_ref(&response)).await {
                send_failures.push(err.to_string());
            }
        }

        if !send_failures.is_empty() {
            return Err(BabataError::internal(format!(
                "Job '{}' finished but failed to send result to {} channel(s): {}",
                self.job_name,
                send_failures.len(),
                send_failures.join("; ")
            )));
        }

        Ok(response)
    }

    fn require_job(&self) -> BabataResult<&JobConfig> {
        let job = self
            .config
            .jobs
            .iter()
            .find(|job| job.name == self.job_name)
            .ok_or_else(|| {
                BabataError::config(format!("Job '{}' not found in config", self.job_name))
            })?;

        if !job.enabled {
            return Err(BabataError::config(format!(
                "Job '{}' is disabled in config",
                self.job_name
            )));
        }

        Ok(job)
    }

    fn require_agent(&self, agent_name: &str) -> BabataResult<&AgentConfig> {
        self.config.get_agent(agent_name).ok_or_else(|| {
            BabataError::config(format!(
                "Agent '{}' not found in config; run onboarding first",
                agent_name
            ))
        })
    }

    fn require_provider_config_for_agent(
        &self,
        agent_config: &AgentConfig,
    ) -> BabataResult<&ProviderConfig> {
        self.config
            .providers
            .iter()
            .find(|provider| provider.matches_name(&agent_config.provider))
            .ok_or_else(|| {
                BabataError::config(format!(
                    "Provider '{}' not found in config",
                    agent_config.provider
                ))
            })
    }

    fn persist_job_history(&self, entry: &JobHistoryEntry) {
        if let Err(err) = self.history_store.insert(entry) {
            error!(
                "Failed to persist job history for '{}' into sqlite: {}",
                entry.job_name, err
            );
        }
    }
}

fn current_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
        .min(i64::MAX as u64) as i64
}

fn elapsed_millis_i64(duration: Duration) -> i64 {
    duration.as_millis().min(i64::MAX as u128) as i64
}
