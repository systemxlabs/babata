use std::{collections::HashMap, sync::Arc, time::Duration};

use backon::{ExponentialBuilder, Retryable};
use log::{error, info, warn};
use tokio::{sync::Mutex, task::JoinHandle};

use crate::{
    BabataResult,
    config::Config,
    error::BabataError,
    message::{Content, Message},
    provider::{GenerationRequest, Provider, build_providers},
    skill::{Skill, load_skills},
    system_prompt::{SystemPromptFile, build_system_prompt, load_system_prompt_files},
    task::{ArtifactEntry, NewTask, TaskStatus, TaskStore},
    tool::{Tool, ToolSpec, build_tools},
};

const PROVIDER_RETRY_MAX_TIMES: usize = 3;
const PROVIDER_RETRY_MIN_DELAY_MS: u64 = 200;
const PROVIDER_RETRY_MAX_DELAY_SECS: u64 = 2;
const TASK_FAILURE_RETRY_DELAY_SECS: u64 = 3;

const TASK_RUNTIME_INSTRUCTIONS: &str = r#"
You are running inside Babata architecture v2.

Execution rules:
- You are responsible for one task at a time.
- `task.md` defines the task goal and completion criteria.
- `progress.md` is the recovery surface. Keep it current when the plan changes, after substantial tool work, before yielding, and before finishing.
- Use the provided tools for side effects. Do not claim a tool succeeded if it failed.
- Use the task `artifacts/` directory for large intermediate or final files when useful.
- Return a normal final assistant response only when the task is actually complete.
"#;

#[derive(Debug)]
pub struct TaskRuntime {
    config: Config,
    store: TaskStore,
    providers: HashMap<String, Arc<dyn Provider>>,
    tools: HashMap<String, Arc<dyn Tool>>,
    system_prompt_files: Vec<SystemPromptFile>,
    skills: Vec<Skill>,
    active_tasks: Mutex<HashMap<String, JoinHandle<()>>>,
    max_steps: usize,
}

impl TaskRuntime {
    pub fn new(config: Config) -> BabataResult<Self> {
        Ok(Self {
            providers: build_providers(&config)?,
            store: TaskStore::open_default()?,
            tools: build_tools(),
            system_prompt_files: load_system_prompt_files()?,
            skills: load_skills()?,
            config,
            active_tasks: Mutex::new(HashMap::new()),
            max_steps: 100,
        })
    }

    pub fn store(&self) -> &TaskStore {
        &self.store
    }

    pub async fn submit_prompt_task(
        self: &Arc<Self>,
        agent_name: &str,
        prompt: Message,
    ) -> BabataResult<String> {
        let agent_config = self.config.get_agent(agent_name)?;
        let task = self.store.create_task(NewTask {
            agent_name: agent_config.name.clone(),
            provider_name: agent_config.provider.clone(),
            model: agent_config.model.clone(),
            task_markdown: build_prompt_task_markdown(&prompt),
            initial_progress: build_initial_progress_markdown(),
            initial_history: vec![prompt],
            parent_task_id: None,
            root_task_id: None,
        })?;
        self.spawn_task(task.task_id.clone()).await?;
        Ok(task.task_id)
    }

    pub async fn submit_task(self: &Arc<Self>, new_task: NewTask) -> BabataResult<String> {
        let task = self.store.create_task(new_task)?;
        self.spawn_task(task.task_id.clone()).await?;
        Ok(task.task_id)
    }

    pub async fn spawn_task(self: &Arc<Self>, task_id: String) -> BabataResult<()> {
        {
            let active = self.active_tasks.lock().await;
            if active.contains_key(&task_id) {
                return Ok(());
            }
        }

        let runtime = Arc::clone(self);
        let task_id_for_handle = task_id.clone();
        let handle = tokio::spawn(async move {
            if let Err(err) = runtime.run_task_loop(task_id_for_handle.clone()).await {
                error!("Task '{}' runtime failed: {}", task_id_for_handle, err);
                if let Err(store_err) = runtime
                    .store
                    .record_error(&task_id_for_handle, Some(&err.to_string()))
                {
                    error!(
                        "Task '{}' failed and error recording also failed: {}",
                        task_id_for_handle, store_err
                    );
                }
            }
            runtime.unregister_task(&task_id_for_handle).await;
        });

        let mut active = self.active_tasks.lock().await;
        active.insert(task_id, handle);
        Ok(())
    }

    pub async fn resume_running_tasks(self: &Arc<Self>) -> BabataResult<()> {
        for task in self.store.list_tasks_by_status(TaskStatus::Running)? {
            self.spawn_task(task.task_id).await?;
        }
        Ok(())
    }

    pub async fn wait_for_task(self: &Arc<Self>, task_id: &str) -> BabataResult<Message> {
        loop {
            let record = self.store.get_task(task_id)?;
            match record.status {
                TaskStatus::Done => {
                    let final_output = record
                        .final_output
                        .or_else(|| self.store.read_final_output(task_id).ok().flatten())
                        .ok_or_else(|| {
                            BabataError::internal(format!(
                                "Task '{}' finished without final output",
                                task_id
                            ))
                        })?;
                    return Ok(Message::AssistantResponse {
                        content: vec![Content::Text { text: final_output }],
                        reasoning_content: None,
                    });
                }
                TaskStatus::Canceled => {
                    return Err(BabataError::internal(format!(
                        "Task '{}' was canceled",
                        task_id
                    )));
                }
                TaskStatus::Paused => {
                    return Err(BabataError::internal(format!(
                        "Task '{}' is paused: {}",
                        task_id,
                        record
                            .last_error
                            .unwrap_or_else(|| "manual intervention required".to_string())
                    )));
                }
                TaskStatus::Running => tokio::time::sleep(Duration::from_millis(200)).await,
            }
        }
    }

    async fn unregister_task(&self, task_id: &str) {
        let mut active = self.active_tasks.lock().await;
        active.remove(task_id);
    }

    async fn run_task_loop(&self, task_id: String) -> BabataResult<()> {
        loop {
            let snapshot = self.store.load_snapshot(&task_id)?;
            if snapshot.record.status != TaskStatus::Running {
                return Ok(());
            }

            let provider = self
                .providers
                .get(&snapshot.record.provider_name)
                .cloned()
                .ok_or_else(|| {
                    BabataError::config(format!(
                        "Task '{}' references unknown provider '{}'",
                        task_id, snapshot.record.provider_name
                    ))
                })?;

            match self.run_react_steps(&task_id, provider).await {
                Ok(()) => return Ok(()),
                Err(err) if should_retry_task_error(&err) => {
                    warn!(
                        "Task '{}' step failed with retryable error: {}",
                        task_id, err
                    );
                    self.store.record_error(&task_id, Some(&err.to_string()))?;
                    tokio::time::sleep(Duration::from_secs(TASK_FAILURE_RETRY_DELAY_SECS)).await;
                }
                Err(err) => {
                    self.store.set_status(
                        &task_id,
                        TaskStatus::Paused,
                        None,
                        Some(&err.to_string()),
                    )?;
                    return Err(err);
                }
            }
        }
    }

    async fn run_react_steps(
        &self,
        task_id: &str,
        provider: Arc<dyn Provider>,
    ) -> BabataResult<()> {
        let tool_specs = self.collect_tool_specs();

        for step_idx in 0..self.max_steps {
            let snapshot = self.store.load_snapshot(task_id)?;
            if snapshot.record.status != TaskStatus::Running {
                return Ok(());
            }

            let system_prompt = self.build_runtime_system_prompt()?;
            let context = build_task_context(&self.store, &snapshot);
            let message = self
                .generate_with_retry(
                    provider.clone(),
                    &system_prompt,
                    &snapshot.record.model,
                    &snapshot.history,
                    &context,
                    &tool_specs,
                )
                .await?;
            self.store
                .append_history_messages(task_id, std::slice::from_ref(&message))?;

            match message {
                Message::AssistantResponse { content, .. } => {
                    let final_output = flatten_text_content(&content);
                    self.store.write_final_output(task_id, &final_output)?;
                    self.store
                        .set_status(task_id, TaskStatus::Done, Some(&final_output), None)?;
                    info!("Task '{}' completed", task_id);
                    return Ok(());
                }
                Message::AssistantToolCalls { calls, .. } => {
                    if calls.is_empty() {
                        return Err(BabataError::provider(format!(
                            "Task '{}' received empty tool calls",
                            task_id
                        )));
                    }

                    let mut tool_results = Vec::with_capacity(calls.len());
                    for call in calls {
                        let tool = self.tools.get(&call.tool_name).ok_or_else(|| {
                            BabataError::tool(format!(
                                "Task '{}' requested unknown tool '{}'",
                                task_id, call.tool_name
                            ))
                        })?;

                        let result = match tool.execute(&call.args).await {
                            Ok(result) => result,
                            Err(err) => format!("Tool execution failed with message: {}", err),
                        };
                        tool_results.push(Message::ToolResult { call, result });
                    }
                    self.store.append_history_messages(task_id, &tool_results)?;
                }
                Message::UserPrompt { .. } | Message::ToolResult { .. } => {
                    return Err(BabataError::provider(format!(
                        "Task '{}' received unsupported provider message type",
                        task_id
                    )));
                }
            }

            if step_idx + 1 == self.max_steps {
                let message = format!(
                    "Task exceeded max step limit ({}). Review progress.md and resume manually.",
                    self.max_steps
                );
                self.store
                    .set_status(task_id, TaskStatus::Paused, None, Some(&message))?;
                return Err(BabataError::internal(message));
            }
        }

        Ok(())
    }

    async fn generate_with_retry(
        &self,
        provider: Arc<dyn Provider>,
        system_prompt: &str,
        model: &str,
        prompts: &[Message],
        context: &[Message],
        tool_specs: &[ToolSpec],
    ) -> BabataResult<Message> {
        let backoff = ExponentialBuilder::default()
            .with_min_delay(Duration::from_millis(PROVIDER_RETRY_MIN_DELAY_MS))
            .with_max_delay(Duration::from_secs(PROVIDER_RETRY_MAX_DELAY_SECS))
            .with_max_times(PROVIDER_RETRY_MAX_TIMES);

        (|| async {
            let response = provider
                .generate(GenerationRequest {
                    system_prompt,
                    model,
                    prompts,
                    context,
                    tools: tool_specs,
                })
                .await?;
            Ok(response.message)
        })
        .retry(backoff)
        .when(|err| matches!(err, BabataError::Provider(_, _)))
        .notify(|err, wait| warn!("Provider generate failed: {}. Retrying in {:?}", err, wait))
        .await
    }

    fn collect_tool_specs(&self) -> Vec<ToolSpec> {
        let mut specs: Vec<ToolSpec> = self
            .tools
            .values()
            .map(|tool| tool.spec().clone())
            .collect();
        specs.sort_by(|a, b| a.name.cmp(&b.name));
        specs
    }

    fn build_runtime_system_prompt(&self) -> BabataResult<String> {
        let base = build_system_prompt(&self.system_prompt_files, &self.skills)?;
        Ok(format!(
            "{}\n\n{}",
            base.trim_end(),
            TASK_RUNTIME_INSTRUCTIONS.trim()
        ))
    }
}

fn should_retry_task_error(err: &BabataError) -> bool {
    matches!(err, BabataError::Provider(_, _) | BabataError::Tool(_, _))
}

fn build_prompt_task_markdown(prompt: &Message) -> String {
    format!(
        r#"# Task

## Goal
Handle the prompt below and produce the best final response.

## Input
{}

## Completion Criteria
- Answer the user request completely.
- Use tools only when they materially help.
- Keep `progress.md` current so the task can resume after restart.
"#,
        render_message_markdown(prompt)
    )
}

fn build_initial_progress_markdown() -> String {
    r#"# Progress

## Current Goal
- Read `task.md`, understand the request, and continue execution.

## Completed
- Task directory initialized.

## Outstanding
- Understand the request.
- Execute the necessary steps.
- Return the final answer.

## Waiting
- None.

## Artifacts
- `artifacts/message_history.json`: persisted task transcript.

## Resume Prompt
Continue from `task.md`, `progress.md`, persisted task history, and any artifacts.
"#
    .to_string()
}

fn build_task_context(store: &TaskStore, snapshot: &crate::task::TaskSnapshot) -> Vec<Message> {
    let task_id = &snapshot.record.task_id;
    let task_dir = store.task_dir(task_id);
    let artifacts_dir = store.artifacts_dir(task_id);
    let artifacts_listing = render_artifacts_listing(&snapshot.artifacts);

    vec![
        Message::UserPrompt {
            content: vec![Content::Text {
                text: format!(
                    r#"Task runtime metadata:
- task_id: {}
- status: {}
- agent: {}
- provider: {}
- model: {}
- task_dir: {}
- task_file: {}
- progress_file: {}
- artifacts_dir: {}

Important:
- `progress.md` is the recovery checkpoint. Update it using file tools when major progress happens.
- Prefer storing large generated content in `artifacts/`.
"#,
                    snapshot.record.task_id,
                    snapshot.record.status.as_str(),
                    snapshot.record.agent_name,
                    snapshot.record.provider_name,
                    snapshot.record.model,
                    task_dir.display(),
                    store.task_path(task_id).display(),
                    store.progress_path(task_id).display(),
                    artifacts_dir.display(),
                ),
            }],
        },
        Message::UserPrompt {
            content: vec![Content::Text {
                text: format!(
                    "Current task definition (`task.md`):\n\n{}",
                    snapshot.task_markdown
                ),
            }],
        },
        Message::UserPrompt {
            content: vec![Content::Text {
                text: format!(
                    "Current progress checkpoint (`progress.md`):\n\n{}",
                    if snapshot.progress_markdown.trim().is_empty() {
                        "(empty)".to_string()
                    } else {
                        snapshot.progress_markdown.clone()
                    }
                ),
            }],
        },
        Message::UserPrompt {
            content: vec![Content::Text {
                text: format!("Current artifacts:\n\n{}", artifacts_listing),
            }],
        },
    ]
}

fn render_artifacts_listing(artifacts: &[ArtifactEntry]) -> String {
    if artifacts.is_empty() {
        return "- No artifacts yet.".to_string();
    }

    artifacts
        .iter()
        .map(|artifact| format!("- {}", artifact.relative_path))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_message_markdown(message: &Message) -> String {
    match message {
        Message::UserPrompt { content } => render_content_markdown(content),
        Message::AssistantResponse { content, .. } => render_content_markdown(content),
        Message::AssistantToolCalls { calls, .. } => calls
            .iter()
            .map(|call| {
                format!(
                    "- tool `{}` with args:\n```json\n{}\n```",
                    call.tool_name, call.args
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Message::ToolResult { result, .. } => result.clone(),
    }
}

fn render_content_markdown(content: &[Content]) -> String {
    let mut lines = Vec::new();
    for part in content {
        match part {
            Content::Text { text } => lines.push(text.clone()),
            Content::ImageUrl { url } => lines.push(format!("[image_url] {}", url)),
            Content::ImageData { media_type, .. } => {
                lines.push(format!("[image_data] {}", media_type.as_mime_str()))
            }
            Content::AudioData { media_type, .. } => {
                lines.push(format!("[audio_data] {}", media_type.as_mime_str()))
            }
        }
    }
    lines.join("\n")
}

fn flatten_text_content(content: &[Content]) -> String {
    content
        .iter()
        .filter_map(|part| match part {
            Content::Text { text } => Some(text.as_str()),
            Content::ImageUrl { .. } | Content::ImageData { .. } | Content::AudioData { .. } => {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}
