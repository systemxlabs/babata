use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use log::{error, info};
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::{
    BabataResult,
    channel::Channel,
    config::{AgentConfig, Config, JobConfig, ProviderConfig},
    error::BabataError,
    message::{Content, Message},
    provider::create_provider,
    skill::{Skill, load_skills},
    system_prompt::{SystemPrompt, load_system_prompts},
    task::AgentTask,
    tool::{Tool, build_tools},
};

use super::{JobHistoryEntry, JobHistoryStore, JobRunStatus};

pub struct JobManager {
    pub config: Config,
    pub job_scheduler: Mutex<Option<JobScheduler>>,
    pub history_store: JobHistoryStore,
    pub channels: Vec<Arc<dyn Channel>>,
    pub tools: HashMap<String, Arc<dyn Tool>>,
    pub system_prompts: Vec<SystemPrompt>,
    pub skills: Vec<Skill>,
}

impl JobManager {
    pub fn new(config: Config, channels: Vec<Arc<dyn Channel>>) -> BabataResult<Self> {
        Ok(Self {
            config,
            job_scheduler: Mutex::new(None),
            history_store: JobHistoryStore::new()?,
            channels,
            tools: build_tools(),
            system_prompts: load_system_prompts()?,
            skills: load_skills()?,
        })
    }

    pub fn require_agent(&self, agent_name: &str) -> BabataResult<&AgentConfig> {
        self.config.get_agent(agent_name).ok_or_else(|| {
            BabataError::config(format!(
                "Agent '{}' not found in config; run onboarding first",
                agent_name
            ))
        })
    }

    pub fn require_provider_config_for_agent(
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

    pub async fn start_scheduler(&self) -> BabataResult<bool> {
        if self.job_scheduler.lock().await.is_some() {
            return Ok(true);
        }

        let enabled_jobs = self
            .config
            .jobs
            .iter()
            .filter(|job| job.enabled)
            .cloned()
            .collect::<Vec<_>>();
        if enabled_jobs.is_empty() {
            return Ok(false);
        }

        let scheduler = JobScheduler::new().await.map_err(|err| {
            BabataError::internal(format!("Failed to initialize job scheduler: {err}"))
        })?;

        for job_config in &enabled_jobs {
            self.register_job(&scheduler, job_config).await?;
        }

        scheduler.start().await.map_err(|err| {
            BabataError::internal(format!("Failed to start job scheduler: {err}"))
        })?;

        *self.job_scheduler.lock().await = Some(scheduler);
        info!(
            "Started job scheduler with {} enabled jobs",
            enabled_jobs.len()
        );
        Ok(true)
    }

    async fn register_job(
        &self,
        scheduler: &JobScheduler,
        job_config: &JobConfig,
    ) -> BabataResult<()> {
        let job_config_payload = serde_json::to_string(job_config).map_err(|err| {
            BabataError::internal(format!(
                "Failed to serialize job config '{}' into JSON: {}",
                job_config.name, err
            ))
        })?;
        let agent_config = self.require_agent(&job_config.agent_name)?;
        let provider_config = self.require_provider_config_for_agent(agent_config)?;
        let provider = create_provider(&agent_config.provider, provider_config)?;

        let scheduler_cron = {
            let trimmed = job_config.cron.trim();
            let field_count = trimmed.split_whitespace().count();
            match field_count {
                5 => format!("0 {trimmed}"),
                6 | 7 => trimmed.to_string(),
                _ => {
                    return Err(BabataError::config(format!(
                        "Unsupported cron expression '{}': expected 5, 6 or 7 fields",
                        job_config.cron
                    )));
                }
            }
        };
        let job_name = job_config.name.clone();
        let job_description = job_config.description.clone();
        let agent_name = job_config.agent_name.clone();
        let prompt = job_config.prompt.clone();
        let model = agent_config.model.clone();
        let job_config_payload = job_config_payload.clone();
        let history_store = self.history_store.clone();
        let channels = self.channels.clone();
        let tools = self.tools.clone();
        let system_prompts = self.system_prompts.clone();
        let skills = self.skills.clone();

        let scheduled_job = Job::new_async(scheduler_cron.as_str(), move |_uuid, _lock| {
            let channels = channels.clone();
            let provider = provider.clone();
            let tools = tools.clone();
            let system_prompts = system_prompts.clone();
            let skills = skills.clone();
            let prompt = prompt.clone();
            let model = model.clone();
            let job_config_payload = job_config_payload.clone();
            let history_store = history_store.clone();
            let job_name = job_name.clone();
            let job_description = job_description.clone();
            let agent_name = agent_name.clone();

            Box::pin(async move {
                info!(
                    "Running scheduled job '{}' (agent='{}'): {}",
                    job_name, agent_name, job_description
                );

                let task = AgentTask::new(
                    vec![Message::UserPrompt {
                        content: vec![Content::Text {
                            text: prompt.clone(),
                        }],
                    }],
                    provider,
                    model.clone(),
                    tools,
                    system_prompts,
                    skills,
                );
                let started_at_epoch = current_epoch_seconds();
                let started_at = Instant::now();

                match task.run().await {
                    Ok(response) => {
                        info!("Scheduled job '{}' completed", job_name);
                        broadcast_job_message(&channels, &job_name, &response).await;
                        persist_job_history(
                            &history_store,
                            &JobHistoryEntry {
                                job_name: job_name.clone(),
                                job_config: job_config_payload.clone(),
                                status: JobRunStatus::Success,
                                response: serde_json::to_string(&response).ok(),
                                error: None,
                                started_at_epoch,
                                finished_at_epoch: current_epoch_seconds(),
                                duration_ms: elapsed_millis_i64(started_at.elapsed()),
                            },
                        );
                    }
                    Err(err) => {
                        let error_message = err.to_string();
                        error!("Scheduled job '{}' failed: {}", job_name, error_message);
                        let failure_message = Message::AssistantResponse {
                            content: vec![Content::Text {
                                text: format!(
                                    "Scheduled job '{}' failed: {}",
                                    job_name, error_message
                                ),
                            }],
                            reasoning_content: None,
                        };
                        broadcast_job_message(&channels, &job_name, &failure_message).await;
                        persist_job_history(
                            &history_store,
                            &JobHistoryEntry {
                                job_name: job_name.clone(),
                                job_config: job_config_payload.clone(),
                                status: JobRunStatus::Failed,
                                response: None,
                                error: Some(error_message),
                                started_at_epoch,
                                finished_at_epoch: current_epoch_seconds(),
                                duration_ms: elapsed_millis_i64(started_at.elapsed()),
                            },
                        );
                    }
                }
            })
        })
        .map_err(|err| {
            BabataError::config(format!(
                "Failed to create scheduled job '{}' with cron '{}': {}",
                job_config.name, job_config.cron, err
            ))
        })?;

        scheduler.add(scheduled_job).await.map_err(|err| {
            BabataError::internal(format!(
                "Failed to register scheduled job '{}': {err}",
                job_config.name
            ))
        })?;

        info!(
            "Registered scheduled job '{}' with cron '{}'",
            job_config.name, job_config.cron
        );
        Ok(())
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

fn persist_job_history(history_store: &JobHistoryStore, entry: &JobHistoryEntry) {
    if let Err(err) = history_store.insert(entry) {
        error!(
            "Failed to persist job history for '{}' into sqlite: {}",
            entry.job_name, err
        );
    }
}

async fn broadcast_job_message(channels: &[Arc<dyn Channel>], job_name: &str, message: &Message) {
    for channel in channels {
        if let Err(err) = channel.send(std::slice::from_ref(message)).await {
            error!(
                "Scheduled job '{}' completed but failed to send result to channel: {}",
                job_name, err
            );
        }
    }
}
