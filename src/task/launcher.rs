use std::{collections::HashMap, sync::Arc};

use log::info;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    BabataResult,
    agent::{Agent, AgentTask, babata::BabataAgent, build_agents},
    channel::Channel,
    config::Config,
    error::BabataError,
    task::{RunningTask, TaskExitEvent, TaskRequest, TaskStore},
};

#[derive(Debug)]
pub struct TaskLauncher {
    agents: HashMap<String, Arc<dyn Agent>>,
    store: TaskStore,
}

impl TaskLauncher {
    pub fn new(
        config: &Config,
        channels: HashMap<String, Arc<dyn Channel>>,
        store: TaskStore,
    ) -> BabataResult<Self> {
        let agents = build_agents(config, channels)?;
        Ok(Self { agents, store })
    }

    pub fn launch(
        &self,
        task_id: Uuid,
        request: &TaskRequest,
        exit_tx: mpsc::Sender<TaskExitEvent>,
    ) -> BabataResult<RunningTask> {
        info!("Launching task {} with request: {:?}", task_id, request);
        let agent_name = match request.agent.as_deref() {
            Some(agent_name) => agent_name,
            None => BabataAgent::name(),
        };

        let agent = self
            .agents
            .get(agent_name)
            .ok_or_else(|| BabataError::config(format!("Agent '{}' not found", agent_name)))?
            .clone();

        let task_record = self.store.get_task(task_id)?;
        let agent_task = AgentTask {
            task_id: task_record.task_id,
            parent_task_id: task_record.parent_task_id,
            root_task_id: task_record.root_task_id,
            prompt: request.prompt.clone(),
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
