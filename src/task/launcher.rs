use std::{collections::HashMap, sync::Arc};

use log::info;
use uuid::Uuid;

use crate::{
    BabataResult,
    agent::{
        Agent,
        babata::{BabataAgent, ToolContext},
        build_agents,
    },
    channel::Channel,
    config::Config,
    error::BabataError,
    task::{RunningTask, TaskRequest, TaskStatus, TaskStore},
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

    pub fn launch(&self, task_id: Uuid, request: &TaskRequest) -> BabataResult<RunningTask> {
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

        let prompt = request.prompt.clone();
        let store = self.store.clone();
        let task_record = self.store.get_task(task_id)?;
        let tool_context = ToolContext {
            task_id: task_record.task_id,
            parent_task_id: task_record.parent_task_id,
            root_task_id: task_record.root_task_id,
        };
        let handle = tokio::spawn(async move {
            match agent.execute(prompt, tool_context).await {
                Ok(_) => {
                    info!("Task {} completed successfully", task_id);
                    if let Err(e) = store.update_task_status(task_id, TaskStatus::Done) {
                        log::error!(
                            "Failed to update status to done for task {}: {}",
                            task_id,
                            e
                        );
                    }
                }
                Err(err) => log::error!("Task {} failed: {}", task_id, err),
            }
        });

        Ok(RunningTask { task_id, handle })
    }
}
