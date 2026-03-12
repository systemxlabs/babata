use std::{collections::HashMap, sync::Arc};

use uuid::Uuid;

use crate::{
    BabataResult,
    agent::{Agent, babata::BabataAgent, build_agents},
    channel::Channel,
    config::Config,
    error::BabataError,
    task::{RunningTask, TaskRequest},
};

#[derive(Debug)]
pub struct TaskLauncher {
    agents: HashMap<String, Arc<dyn Agent>>,
}

impl TaskLauncher {
    pub fn new(config: &Config, channels: HashMap<String, Arc<dyn Channel>>) -> BabataResult<Self> {
        let agents = build_agents(config, channels)?;
        Ok(Self { agents })
    }

    pub fn launch(&self, task_id: Uuid, request: &TaskRequest) -> BabataResult<RunningTask> {
        let agent_name = match request.agent.as_deref() {
            Some(agent_name) => agent_name,
            None => BabataAgent::name(),
        };

        let agent = self
            .agents
            .get(agent_name)
            .ok_or_else(|| BabataError::config(format!("Agent '{}' not found", agent_name)))?
            .clone();

        let prompt = request.prompt.clone();
        let handle = tokio::spawn(async move {
            if let Err(err) = agent.execute(prompt).await {
                log::error!("Task {} failed: {}", task_id, err);
            }
        });

        Ok(RunningTask { task_id, handle })
    }
}
