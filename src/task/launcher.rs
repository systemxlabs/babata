use std::{collections::HashMap, sync::Arc};

use log::info;
use tokio::sync::mpsc;

use crate::{
    BabataResult,
    agent::{Agent, AgentTask, babata::BabataAgent, build_agents},
    channel::Channel,
    config::Config,
    error::BabataError,
    task::{RunningTask, TaskExitEvent, TaskRecord},
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

    pub fn launch(
        &self,
        task: &TaskRecord,
        exit_tx: mpsc::Sender<TaskExitEvent>,
    ) -> BabataResult<RunningTask> {
        info!(
            "Launching task {} with task record: {:?}",
            task.task_id, task
        );
        let agent_name = match task.agent.as_deref() {
            Some(agent_name) => agent_name,
            None => BabataAgent::name(),
        };

        let agent = self
            .agents
            .get(agent_name)
            .ok_or_else(|| BabataError::config(format!("Agent '{}' not found", agent_name)))?
            .clone();

        let task_id = task.task_id;
        let agent_task = AgentTask {
            task_id,
            parent_task_id: task.parent_task_id,
            root_task_id: task.root_task_id,
            prompt: task.prompt.clone(),
        };
        let handle = tokio::spawn(async move {
            let result = agent.execute(agent_task).await;
            let event = match result {
                Ok(()) => TaskExitEvent::Completed { task_id },
                Err(error) => TaskExitEvent::Failed { task_id, error },
            };
            let _ = exit_tx.send(event).await;
        });

        Ok(RunningTask { task_id, handle })
    }
}
