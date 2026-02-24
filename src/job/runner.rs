use crate::{
    BabataResult,
    channel::build_channels,
    config::{AgentConfig, Config, JobConfig, ProviderConfig},
    error::BabataError,
    message::{Content, Message},
    provider::create_provider,
    skill::load_skills,
    system_prompt::load_system_prompts,
    task::AgentTask,
    tool::build_tools,
};

pub struct JobRunner {
    pub config: Config,
    pub job_name: String,
}

impl JobRunner {
    pub fn new(config: Config, job_name: String) -> Self {
        Self { config, job_name }
    }

    pub async fn run(&self) -> BabataResult<()> {
        let job_config = self.require_job()?;
        let agent_config = self.require_agent(&job_config.agent_name)?;
        let provider_config = self.require_provider_config_for_agent(agent_config)?;
        let provider = create_provider(&agent_config.provider, provider_config)?;

        let task = AgentTask::new(
            vec![Message::UserPrompt {
                content: vec![Content::Text {
                    text: job_config.prompt.clone(),
                }],
            }],
            provider,
            agent_config.model.clone(),
            build_tools(),
            load_system_prompts()?,
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

        Ok(())
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
}
