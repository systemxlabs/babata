use std::{collections::HashMap, path::Path, sync::Arc};

use tokio::{fs as tokio_fs, sync::mpsc, task::JoinHandle};
use uuid::Uuid;

use crate::{
    BabataResult,
    agent::{AgentTask, load_agent_by_name},
    channel::Channel,
    memory::Memory,
    message::Content,
    task::{RunningTask, SteerQueue, TaskExitEvent, TaskRecord},
    task_info, task_warn,
    tool::{Tool, build_tools},
    utils::task_dir,
};

#[derive(Debug)]
pub struct TaskLauncher {
    pub all_tools: HashMap<String, Arc<dyn Tool>>,
}

impl TaskLauncher {
    pub fn new(channels: HashMap<String, Arc<dyn Channel>>) -> BabataResult<Self> {
        let all_tools = build_tools(channels)?;
        Ok(Self { all_tools })
    }

    pub fn launch(
        &self,
        task: &TaskRecord,
        prompt: Vec<Content>,
        exit_tx: mpsc::Sender<TaskExitEvent>,
    ) -> BabataResult<RunningTask> {
        task_info!(
            task.task_id,
            "Launching task with task record: {:?} and prompt: {:?}",
            task,
            prompt
        );
        let agent = load_agent_by_name(&task.agent)?;
        let memory = Memory::new(agent.home()?)?;

        let steer_queue = SteerQueue::default();

        let task_id = task.task_id;
        let prompt = build_launch_prompt(task_id, prompt)?;
        let mut agent_task = AgentTask {
            task_id,
            parent_task_id: task.parent_task_id,
            root_task_id: task.root_task_id,
            prompt,
            agent,
            memory,
            all_tools: self.all_tools.clone(),
            steer_queue: Some(steer_queue.clone()),
        };
        let handle = tokio::spawn(async move {
            let result = agent_task.run().await;
            let event = match result {
                Ok(content) => {
                    if let Err(error) = write_final_response(task_id, &content).await {
                        task_warn!(task_id, "Failed to persist final response: {}", error);
                    }
                    TaskExitEvent::Completed { task_id }
                }
                Err(error) => TaskExitEvent::Failed { task_id, error },
            };
            let _ = exit_tx.send(event).await;
        });

        Ok(RunningTask {
            task_id,
            handle,
            steer_queue,
            collaboration_handle: None,
        })
    }

    pub fn relaunch(
        &self,
        task: &TaskRecord,
        exit_tx: mpsc::Sender<TaskExitEvent>,
        reason: &str,
    ) -> BabataResult<RunningTask> {
        task_info!(
            task.task_id,
            "Relaunching task with reason '{}' and task record: {:?}",
            reason,
            task
        );
        let agent = load_agent_by_name(&task.agent)?;
        let memory = Memory::new(agent.home()?)?;

        let steer_queue = SteerQueue::default();

        let task_id = task.task_id;
        let prompt = build_relaunch_prompt(task_id, reason)?;
        let mut agent_task = AgentTask {
            task_id,
            parent_task_id: task.parent_task_id,
            root_task_id: task.root_task_id,
            prompt,
            agent,
            memory,
            all_tools: self.all_tools.clone(),
            steer_queue: Some(steer_queue.clone()),
        };
        let handle = tokio::spawn(async move {
            let result = agent_task.run().await;
            let event = match result {
                Ok(content) => {
                    if let Err(error) = write_final_response(task_id, &content).await {
                        task_warn!(task_id, "Failed to persist final response: {}", error);
                    }
                    TaskExitEvent::Completed { task_id }
                }
                Err(error) => TaskExitEvent::Failed { task_id, error },
            };
            let _ = exit_tx.send(event).await;
        });

        Ok(RunningTask {
            task_id,
            handle,
            steer_queue,
            collaboration_handle: None,
        })
    }

    pub fn collaborate(
        &self,
        task: &TaskRecord,
        agent_name: &str,
        collaboration_prompt: &str,
    ) -> BabataResult<JoinHandle<BabataResult<Vec<Content>>>> {
        let agent = load_agent_by_name(agent_name)?;
        let memory = Memory::new(agent.home()?)?;

        let prompt = build_collaboration_prompt(task.task_id, collaboration_prompt)?;

        let mut agent_task = AgentTask {
            task_id: task.task_id,
            parent_task_id: task.parent_task_id,
            root_task_id: task.root_task_id,
            prompt,
            agent,
            memory,
            all_tools: self.all_tools.clone(),
            steer_queue: None,
        };

        Ok(tokio::spawn(async move { agent_task.run().await }))
    }
}

fn build_launch_prompt(task_id: Uuid, mut prompt: Vec<Content>) -> BabataResult<Vec<Content>> {
    prompt.insert(
        0,
        Content::Text {
            text: format!("Execute task (id: {task_id}) with prompt"),
        },
    );

    Ok(prompt)
}

fn build_relaunch_prompt(task_id: Uuid, relaunch_reason: &str) -> BabataResult<Vec<Content>> {
    let mut prompt = Vec::with_capacity(2);
    prompt.push(Content::Text {
        text: format!("This task (id: {task_id}) is relaunched with reason: {relaunch_reason}. Please read the files in the task's home directory and continue executing the task."),
    });

    Ok(prompt)
}

fn build_collaboration_prompt(
    task_id: Uuid,
    collaboration_prompt: &str,
) -> BabataResult<Vec<Content>> {
    let mut prompt = Vec::with_capacity(2);
    prompt.push(Content::Text {
        text: format!(
            "You are collaborating on task (id: {task_id}) with request: {collaboration_prompt}"
        ),
    });

    Ok(prompt)
}

async fn write_final_response(task_id: Uuid, content: &[Content]) -> BabataResult<()> {
    let task_dir = task_dir(task_id)?;
    write_final_response_in(&task_dir, content).await
}

async fn write_final_response_in(task_dir: &Path, content: &[Content]) -> BabataResult<()> {
    tokio_fs::create_dir_all(task_dir).await?;
    tokio_fs::write(
        task_dir.join("final-response.md"),
        render_final_response_markdown(content),
    )
    .await?;
    Ok(())
}

fn render_final_response_markdown(content: &[Content]) -> String {
    let rendered = content
        .iter()
        .map(render_content_as_markdown)
        .collect::<Vec<_>>()
        .join("\n\n");
    if rendered.is_empty() {
        String::new()
    } else {
        format!("{rendered}\n")
    }
}

fn render_content_as_markdown(content: &Content) -> String {
    match content {
        Content::Text { text } => text.clone(),
        Content::ImageUrl { url } => format!("![image]({url})"),
        Content::ImageData { media_type, .. } => {
            format!("[embedded image data omitted: {media_type}]")
        }
        Content::AudioData { media_type, .. } => {
            format!("[embedded audio data omitted: {media_type}]")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::write_final_response_in;
    use crate::message::{Content, MediaType};
    use std::fs;
    use uuid::Uuid;

    #[tokio::test]
    async fn write_final_response_creates_markdown_file() {
        let task_dir = std::env::temp_dir().join(format!("babata-launcher-{}", Uuid::new_v4()));
        let content = vec![
            Content::Text {
                text: "Task finished successfully.".to_string(),
            },
            Content::ImageUrl {
                url: "https://example.com/image.png".to_string(),
            },
            Content::ImageData {
                data: "ignored".to_string(),
                media_type: MediaType::ImagePng,
            },
            Content::AudioData {
                data: "ignored".to_string(),
                media_type: MediaType::AudioMp3,
            },
        ];

        write_final_response_in(&task_dir, &content)
            .await
            .expect("write final response");

        let final_response = fs::read_to_string(task_dir.join("final-response.md"))
            .expect("read final response file");
        assert_eq!(
            final_response,
            "Task finished successfully.\n\n![image](https://example.com/image.png)\n\n[embedded image data omitted: image/png]\n\n[embedded audio data omitted: audio/mp3]\n"
        );

        let _ = fs::remove_dir_all(&task_dir);
    }
}
