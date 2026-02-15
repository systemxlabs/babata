use std::{collections::HashMap, sync::Arc};

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

pub struct JobManager {
    pub config: Config,
    pub job_scheduler: Mutex<Option<JobScheduler>>,
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
        let agent_config = self.require_agent(&job_config.agent_name)?;
        let provider_config = self.require_provider_config_for_agent(agent_config)?;
        let provider = create_provider(&agent_config.provider, provider_config)?;

        let scheduler_cron = normalize_scheduler_cron(&job_config.cron)?;
        let job_name = job_config.name.clone();
        let job_description = job_config.description.clone();
        let agent_name = job_config.agent_name.clone();
        let prompt = job_config.prompt.clone();
        let model = agent_config.model.clone();
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
                        content: vec![Content::Text { text: prompt }],
                    }],
                    provider,
                    model,
                    tools,
                    system_prompts,
                    skills,
                );

                match task.run().await {
                    Ok(response) => {
                        info!("Scheduled job '{}' completed", job_name);
                        broadcast_job_message(&channels, &job_name, &response).await;
                    }
                    Err(err) => {
                        error!("Scheduled job '{}' failed: {}", job_name, err);
                        let failure_message = Message::AssistantResponse {
                            content: vec![Content::Text {
                                text: format!("Scheduled job '{}' failed: {}", job_name, err),
                            }],
                        };
                        broadcast_job_message(&channels, &job_name, &failure_message).await;
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

fn normalize_scheduler_cron(cron: &str) -> BabataResult<String> {
    let trimmed = cron.trim();
    let field_count = trimmed.split_whitespace().count();
    match field_count {
        5 => Ok(format!("0 {trimmed}")),
        6 | 7 => Ok(trimmed.to_string()),
        _ => Err(BabataError::config(format!(
            "Unsupported cron expression '{}': expected 5, 6 or 7 fields",
            cron
        ))),
    }
}
