use std::{
    fs::File,
    path::{Path, PathBuf},
    process::Stdio,
};

use log::info;
use tokio::{io::AsyncWriteExt, process::Command};
use uuid::Uuid;

use crate::{
    BabataResult,
    agent::{Agent, AgentTask},
    config::{AgentConfig, CodexAgentConfig, Config},
    error::BabataError,
    message::Content,
    task::task_dir,
};

#[derive(Debug, Clone)]
pub struct CodexAgent {
    command: String,
    workspace: PathBuf,
    model: Option<String>,
}

impl CodexAgent {
    pub fn new(config: &Config) -> BabataResult<Self> {
        let agent_config = config.get_agent(CodexAgent::name())?;
        let AgentConfig::Codex(codex_config) = agent_config else {
            return Err(BabataError::config(
                "Agent config for 'codex' must be of type 'CodexAgentConfig'",
            ));
        };

        Ok(Self::from_config(codex_config))
    }

    fn from_config(config: &CodexAgentConfig) -> Self {
        Self {
            command: config.command.clone(),
            workspace: PathBuf::from(&config.workspace),
            model: config.model.clone(),
        }
    }

    fn build_prompt(&self, task: &AgentTask) -> BabataResult<String> {
        let prompt_text = prompt_to_text(&task.prompt)?;
        let parent_task_id = task
            .parent_task_id
            .map(|task_id| task_id.to_string())
            .unwrap_or_else(|| "none".to_string());

        Ok(format!(
            "You are executing a Babata task through Codex CLI.\n\
             \n\
             Task metadata:\n\
             - task_id: {}\n\
             - parent_task_id: {}\n\
             - root_task_id: {}\n\
             - default workspace: {}\n\
             \n\
             Execution mode:\n\
             - You are running with approvals and sandbox bypassed.\n\
             - Use the workspace above as the default working root.\n\
             - You may access other local files when necessary.\n\
             - Prefer absolute paths when referring to files outside the workspace.\n\
             \n\
             User task:\n\
             {}",
            task.task_id,
            parent_task_id,
            task.root_task_id,
            self.workspace.display(),
            prompt_text
        ))
    }

    fn output_paths(&self, task_id: Uuid) -> BabataResult<CodexOutputPaths> {
        let task_dir = task_dir(task_id)?;
        Ok(CodexOutputPaths {
            last_message: task_dir.join("codex-last-message.md"),
            stdout: task_dir.join("codex-stdout.log"),
            stderr: task_dir.join("codex-stderr.log"),
        })
    }
}

#[async_trait::async_trait]
impl Agent for CodexAgent {
    fn name() -> &'static str {
        "codex"
    }

    fn description() -> &'static str {
        "Use for code writing and code review tasks"
    }

    async fn execute(&self, task: AgentTask) -> BabataResult<()> {
        let prompt = self.build_prompt(&task)?;
        let output_paths = self.output_paths(task.task_id)?;
        info!(
            "Executing task {} with Codex CLI in workspace {}",
            task.task_id,
            self.workspace.display()
        );

        let mut command = Command::new(&self.command);
        command
            .arg("exec")
            .arg("--cd")
            .arg(&self.workspace)
            .arg("--skip-git-repo-check")
            .arg("--dangerously-bypass-approvals-and-sandbox")
            .arg("--color")
            .arg("never")
            .arg("--output-last-message")
            .arg(&output_paths.last_message)
            .kill_on_drop(true);

        if let Some(model) = &self.model {
            command.arg("--model").arg(model);
        }

        let stdout = File::create(&output_paths.stdout).map_err(|err| {
            BabataError::internal(format!(
                "Failed to create Codex stdout log '{}': {}",
                output_paths.stdout.display(),
                err
            ))
        })?;
        let stderr = File::create(&output_paths.stderr).map_err(|err| {
            BabataError::internal(format!(
                "Failed to create Codex stderr log '{}': {}",
                output_paths.stderr.display(),
                err
            ))
        })?;

        command
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr));

        let mut child = command.spawn().map_err(|err| {
            BabataError::internal(format!(
                "Failed to start Codex CLI '{}': {}",
                self.command, err
            ))
        })?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| BabataError::internal("Failed to open Codex CLI stdin"))?;
        stdin.write_all(prompt.as_bytes()).await.map_err(|err| {
            BabataError::internal(format!(
                "Failed to write prompt to Codex CLI stdin: {}",
                err
            ))
        })?;
        stdin.shutdown().await.map_err(|err| {
            BabataError::internal(format!("Failed to close Codex CLI stdin: {}", err))
        })?;
        drop(stdin);

        let status = child.wait().await.map_err(|err| {
            BabataError::internal(format!("Failed to wait for Codex CLI process: {}", err))
        })?;

        if status.success() {
            return Ok(());
        }

        let mut details = Vec::new();
        if let Some(code) = status.code() {
            details.push(format!("exit code {code}"));
        } else {
            details.push("terminated by signal".to_string());
        }
        let last_message = read_trimmed_file(&output_paths.last_message)?;
        let stdout_log = read_trimmed_file(&output_paths.stdout)?;
        let stderr_log = read_trimmed_file(&output_paths.stderr)?;
        if !last_message.is_empty() {
            details.push(format!("last message: {last_message}"));
        }
        if !stdout_log.is_empty() {
            details.push(format!("stdout log: {stdout_log}"));
        }
        if !stderr_log.is_empty() {
            details.push(format!("stderr log: {stderr_log}"));
        }

        Err(BabataError::internal(format!(
            "Codex CLI task execution failed ({})",
            details.join(", ")
        )))
    }
}

#[derive(Debug)]
struct CodexOutputPaths {
    last_message: PathBuf,
    stdout: PathBuf,
    stderr: PathBuf,
}

fn read_trimmed_file(path: &Path) -> BabataResult<String> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(String::new()),
        Err(err) => {
            return Err(BabataError::internal(format!(
                "Failed to read file '{}': {}",
                path.display(),
                err
            )));
        }
    };

    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }

    const MAX_CHARS: usize = 4_000;
    if trimmed.chars().count() <= MAX_CHARS {
        return Ok(trimmed.to_string());
    }

    let truncated: String = trimmed.chars().take(MAX_CHARS).collect();
    Ok(format!("{truncated}..."))
}

fn prompt_to_text(prompt: &[Content]) -> BabataResult<String> {
    let mut parts = Vec::with_capacity(prompt.len());
    for content in prompt {
        match content {
            Content::Text { text } => parts.push(text.as_str()),
            Content::ImageUrl { .. } | Content::ImageData { .. } | Content::AudioData { .. } => {
                return Err(BabataError::config(
                    "Codex agent only supports text prompt content",
                ));
            }
        }
    }

    Ok(parts.join("\n\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::Content;
    use uuid::Uuid;

    #[test]
    fn prompt_to_text_rejects_non_text_content() {
        let prompt = vec![Content::ImageUrl {
            url: "https://example.com/image.png".to_string(),
        }];

        let err = prompt_to_text(&prompt).expect_err("non-text prompt should fail");
        assert!(err.to_string().contains("only supports text"));
    }

    #[test]
    fn build_prompt_includes_task_metadata_and_workspace() {
        let agent = CodexAgent {
            command: "codex".to_string(),
            workspace: PathBuf::from("C:/workspace/project"),
            model: Some("codex-mini-latest".to_string()),
        };
        let task = AgentTask {
            task_id: Uuid::nil(),
            parent_task_id: None,
            root_task_id: Uuid::nil(),
            prompt: vec![Content::Text {
                text: "Fix failing tests".to_string(),
            }],
        };

        let prompt = agent.build_prompt(&task).expect("build prompt");
        assert!(prompt.contains("task_id: 00000000-0000-0000-0000-000000000000"));
        assert!(prompt.contains("default workspace: C:/workspace/project"));
        assert!(prompt.contains("Fix failing tests"));
    }
}
