use std::{collections::HashMap, sync::Arc};

use log::info;
use tokio::{sync::mpsc, task::JoinHandle};
use uuid::Uuid;

use crate::{
    BabataResult,
    agent::{Agent, AgentTask},
    channel::Channel,
    error::BabataError,
    memory::Memory,
    message::Content,
    task::{RunningTask, SteerMessage, TaskExitEvent, TaskRecord},
    tool::{Tool, build_tools},
    utils::task_dir,
};

#[derive(Debug)]
pub struct TaskLauncher {
    default_agent: Arc<Agent>,
    agents: HashMap<String, Arc<Agent>>,
    memories: HashMap<String, Arc<Memory>>,
    all_tools: HashMap<String, Arc<dyn Tool>>,
}

impl TaskLauncher {
    pub fn new(
        agents: HashMap<String, Arc<Agent>>,
        channels: HashMap<String, Arc<dyn Channel>>,
    ) -> BabataResult<Self> {
        let default_agent = agents
            .values()
            .find(|agent| matches!(agent.frontmatter.default, Some(true)))
            .ok_or(BabataError::internal("No default agent"))?
            .clone();
        let mut memories = HashMap::with_capacity(agents.len());
        for (name, agent) in &agents {
            let memory = Memory::new(agent.home()?)?;
            memories.insert(name.clone(), Arc::new(memory));
        }
        let all_tools = build_tools(channels)?;
        Ok(Self {
            default_agent,
            agents,
            memories,
            all_tools,
        })
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
        self.launch_internal(task, exit_tx, None)
    }

    pub fn relaunch(
        &self,
        task: &TaskRecord,
        exit_tx: mpsc::Sender<TaskExitEvent>,
        reason: &str,
    ) -> BabataResult<RunningTask> {
        info!(
            "Relaunching task {} with reason '{}' and task record: {:?}",
            task.task_id, reason, task
        );
        self.launch_internal(task, exit_tx, Some(reason))
    }

    pub fn collaborate(
        &self,
        task: &TaskRecord,
        agent_name: &str,
        collaboration_prompt: &str,
    ) -> BabataResult<JoinHandle<BabataResult<Vec<Content>>>> {
        let agent = self
            .agents
            .get(agent_name)
            .ok_or_else(|| BabataError::not_found(format!("Agent '{}' not found", agent_name)))?
            .clone();
        let memory = self
            .memories
            .get(agent_name)
            .ok_or_else(|| BabataError::config(format!("Agent memory '{}' not found", agent_name)))?
            .clone();

        let prompt = build_collaboration_prompt(task.task_id, collaboration_prompt)?;

        let mut agent_task = AgentTask {
            task_id: task.task_id,
            parent_task_id: task.parent_task_id,
            root_task_id: task.root_task_id,
            prompt,
            agent,
            memory,
            all_tools: self.all_tools.clone(),
            steer_rx: None,
        };

        Ok(tokio::spawn(async move { agent_task.run().await }))
    }

    fn launch_internal(
        &self,
        task: &TaskRecord,
        exit_tx: mpsc::Sender<TaskExitEvent>,
        reason: Option<&str>,
    ) -> BabataResult<RunningTask> {
        let agent = match task.agent.as_deref() {
            Some(agent_name) => self
                .agents
                .get(agent_name)
                .ok_or_else(|| BabataError::not_found(format!("Agent '{}' not found", agent_name)))?
                .clone(),
            None => self.default_agent.clone(),
        };

        let memory = self
            .memories
            .get(&agent.frontmatter.name)
            .ok_or_else(|| {
                BabataError::config(format!(
                    "Agent memory '{}' not found",
                    agent.frontmatter.name
                ))
            })?
            .clone();

        // Create steer channel
        let (steer_tx, steer_rx) = mpsc::channel::<SteerMessage>(128);

        let task_id = task.task_id;
        let prompt = match reason {
            Some(reason) => build_relaunch_prompt(task_id, reason)?,
            None => build_launch_prompt(task_id)?,
        };
        let agent_task = AgentTask {
            task_id,
            parent_task_id: task.parent_task_id,
            root_task_id: task.root_task_id,
            prompt,
            agent,
            memory,
            all_tools: self.all_tools.clone(),
            steer_rx: Some(steer_rx),
        };
        let handle = tokio::spawn(async move {
            let mut agent_task = agent_task;
            let result = agent_task.run().await;
            let event = match result {
                Ok(_) => TaskExitEvent::Completed { task_id },
                Err(error) => TaskExitEvent::Failed { task_id, error },
            };
            let _ = exit_tx.send(event).await;
        });

        Ok(RunningTask {
            task_id,
            handle,
            steer_tx,
            collaboration_handle: None,
        })
    }
}

fn build_relaunch_prompt(task_id: uuid::Uuid, relaunch_reason: &str) -> BabataResult<Vec<Content>> {
    let task_dir = task_dir(task_id)?;
    let task_md_path = task_dir.join("task.md");
    let progress_md_path = task_dir.join("progress.md");

    let task_markdown = std::fs::read_to_string(&task_md_path)?;
    let progress_markdown = std::fs::read_to_string(&progress_md_path)?;

    let mut prompt = Vec::with_capacity(2);
    prompt.push(Content::Text {
        text: format!("This task is relaunched with reason: {relaunch_reason}"),
    });

    prompt.push(Content::Text {
        text: format!(
            r#"Current task state is defined by the following files.
- `task.md` at {}
- `progress.md` at {}

Below is the content of `task.md`
{}

---

Below is the content of `progress.md`
{}"#,
            task_md_path.display(),
            progress_md_path.display(),
            task_markdown,
            progress_markdown
        ),
    });

    Ok(prompt)
}

fn build_launch_prompt(task_id: uuid::Uuid) -> BabataResult<Vec<Content>> {
    let task_dir = task_dir(task_id)?;
    let task_md_path = task_dir.join("task.md");

    let task_markdown = std::fs::read_to_string(&task_md_path)?;

    let prompt = vec![Content::Text {
        text: format!(
            r#"Execute task (from {}) below:

{}"#,
            task_md_path.display(),
            task_markdown,
        ),
    }];

    Ok(prompt)
}

fn build_collaboration_prompt(
    task_id: Uuid,
    collaboration_prompt: &str,
) -> BabataResult<Vec<Content>> {
    let task_dir = task_dir(task_id)?;
    let task_md_path = task_dir.join("task.md");
    let progress_md_path = task_dir.join("progress.md");

    let task_markdown = std::fs::read_to_string(&task_md_path)?;
    let progress_markdown = std::fs::read_to_string(&progress_md_path)?;

    let mut prompt = Vec::with_capacity(2);

    prompt.push(Content::Text {
        text: format!(
            "You are collaborating on the current task with request: {collaboration_prompt}"
        ),
    });

    prompt.push(Content::Text {
        text: format!(
            r#"Current task state is defined by the following files.
- `task.md` at {}
- `progress.md` at {}

Below is the content of `task.md`
{}

---

Below is the content of `progress.md`
{}"#,
            task_md_path.display(),
            progress_md_path.display(),
            task_markdown,
            progress_markdown
        ),
    });

    Ok(prompt)
}
