use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use log::{error, info};

use crate::{
    BabataResult,
    channel::build_channels,
    config::{AgentConfig, Config, JobConfig, ProviderConfig},
    error::BabataError,
    memory::Memory,
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

        let user_message = Message::UserPrompt {
            content: vec![Content::Text {
                text: job_config.prompt.clone(),
            }],
        };
        info!("Job prompt: {}", job_config.prompt);

        let task = AgentTask::new(
            vec![user_message.clone()],
            provider,
            agent_config.model.clone(),
            build_tools(),
            load_system_prompt_files()?,
            load_skills()?,
        );

        let response = task.run().await?;
        info!("Job result: {:?}", response);
        if is_none_text_response(&response) {
            info!(
                "Job '{}' result is literal 'None'; skipping channel notification",
                self.job_name
            );
            return Ok(response);
        }

        self.persist_job_memory(&[user_message, response.clone()]);

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

    fn persist_job_memory(&self, messages: &[Message]) {
        let memory = match Memory::new() {
            Ok(memory) => memory,
            Err(err) => {
                error!(
                    "Failed to open memory store for job '{}'; skipping memory insert: {}",
                    self.job_name, err
                );
                return;
            }
        };

        if let Err(err) = memory.insert_messages(messages) {
            error!(
                "Failed to insert {} message(s) into memory for job '{}': {}",
                messages.len(),
                self.job_name,
                err
            );
        }
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

fn is_none_text_response(message: &Message) -> bool {
    let Message::AssistantResponse { content, .. } = message else {
        return false;
    };

    let mut saw_text = false;
    for part in content {
        match part {
            Content::Text { text } => {
                saw_text = true;
                if text != "None" {
                    return false;
                }
            }
            Content::ImageUrl { .. } | Content::ImageData { .. } | Content::AudioData { .. } => {
                return false;
            }
        }
    }

    saw_text
}

#[cfg(test)]
mod tests {
    use crate::message::{Content, Message};

    use super::is_none_text_response;

    #[test]
    fn is_none_text_response_returns_true_for_literal_none_text() {
        let message = Message::AssistantResponse {
            content: vec![Content::Text {
                text: "None".to_string(),
            }],
            reasoning_content: None,
        };

        assert!(is_none_text_response(&message));
    }

    #[test]
    fn is_none_text_response_returns_false_for_non_none_text() {
        let message = Message::AssistantResponse {
            content: vec![Content::Text {
                text: "done".to_string(),
            }],
            reasoning_content: None,
        };

        assert!(!is_none_text_response(&message));
    }

    #[test]
    fn is_none_text_response_returns_false_for_non_assistant_response() {
        let message = Message::UserPrompt {
            content: vec![Content::Text {
                text: "None".to_string(),
            }],
        };

        assert!(!is_none_text_response(&message));
    }

    #[test]
    fn is_none_text_response_returns_false_when_images_exist() {
        let message = Message::AssistantResponse {
            content: vec![
                Content::Text {
                    text: "None".to_string(),
                },
                Content::ImageUrl {
                    url: "https://example.com/image.png".to_string(),
                },
            ],
            reasoning_content: None,
        };

        assert!(!is_none_text_response(&message));
    }
}
