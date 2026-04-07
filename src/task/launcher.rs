use std::{collections::HashMap, sync::Arc};

use log::info;
use tokio::sync::mpsc;

use crate::{
    BabataResult,
    agent::{Agent, AgentDefinition, AgentTask, build_agents},
    channel::Channel,
    config::Config,
    error::BabataError,
    message::Content,
    task::{RunningTask, TaskExitEvent, TaskRecord, task_dir},
};

#[derive(Debug)]
pub struct TaskLauncher {
    default_agent: Arc<Agent>,
    agents: HashMap<String, Arc<Agent>>,
}

impl TaskLauncher {
    pub fn new(
        config: &Config,
        agent_definitions: &[AgentDefinition],
        channels: HashMap<String, Arc<dyn Channel>>,
    ) -> BabataResult<Self> {
        let agents = build_agents(config, agent_definitions, channels)?;
        let default_agent = agents
            .values()
            .find(|agent| matches!(agent.definition.frontmatter.default, Some(true)))
            .ok_or(BabataError::internal("No default agent"))?
            .clone();
        Ok(Self {
            default_agent,
            agents,
        })
    }

    pub fn launch(
        &self,
        task: &TaskRecord,
        exit_tx: mpsc::Sender<TaskExitEvent>,
    ) -> BabataResult<RunningTask> {
        self.launch_internal(task, exit_tx, None)
    }

    pub fn relaunch(
        &self,
        task: &TaskRecord,
        exit_tx: mpsc::Sender<TaskExitEvent>,
        reason: &str,
    ) -> BabataResult<RunningTask> {
        self.launch_internal(task, exit_tx, Some(reason))
    }

    fn launch_internal(
        &self,
        task: &TaskRecord,
        exit_tx: mpsc::Sender<TaskExitEvent>,
        reason: Option<&str>,
    ) -> BabataResult<RunningTask> {
        match reason {
            Some(reason) => info!(
                "Relaunching task {} with reason '{}' and task record: {:?}",
                task.task_id, reason, task
            ),
            None => info!(
                "Launching task {} with task record: {:?}",
                task.task_id, task
            ),
        }
        let agent_name = match task.agent.as_deref() {
            Some(agent_name) => agent_name,
            None => &self.default_agent.definition.frontmatter.name,
        };

        let agent = self
            .agents
            .get(agent_name)
            .ok_or_else(|| BabataError::config(format!("Agent '{}' not found", agent_name)))?
            .clone();

        let task_id = task.task_id;
        let prompt = build_task_prompt(task.task_id, reason)?;
        let agent_task = AgentTask {
            task_id,
            parent_task_id: task.parent_task_id,
            root_task_id: task.root_task_id,
            prompt,
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

fn build_task_prompt(
    task_id: uuid::Uuid,
    relaunch_reason: Option<&str>,
) -> BabataResult<Vec<Content>> {
    let task_dir = task_dir(task_id)?;
    let task_md_path = task_dir.join("task.md");
    let progress_md_path = task_dir.join("progress.md");

    let task_markdown = std::fs::read_to_string(&task_md_path).map_err(|err| {
        BabataError::internal(format!(
            "Failed to read task file '{}': {}",
            task_md_path.display(),
            err
        ))
    })?;
    let progress_markdown = std::fs::read_to_string(&progress_md_path).map_err(|err| {
        BabataError::internal(format!(
            "Failed to read progress file '{}': {}",
            progress_md_path.display(),
            err
        ))
    })?;

    let mut prompt = Vec::with_capacity(2);
    if let Some(reason) = relaunch_reason {
        prompt.push(Content::Text {
            text: format!(
                r#"This task is being relaunched.
Relaunch reason: {}"#,
                reason
            ),
        });
    }

    prompt.push(Content::Text {
        text: format!(
            r#"Current task state is defined by the following files.

`task.md` path: {}
`progress.md` path: {}

Below is the content of `task.md`
{}

---

Below is the content of `progress.md`
{}
"#,
            task_md_path.display(),
            progress_md_path.display(),
            task_markdown,
            progress_markdown
        ),
    });

    Ok(prompt)
}
