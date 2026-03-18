use std::{path::PathBuf, process::Stdio};

use log::info;
use tokio::{io::AsyncWriteExt, process::Command};

use crate::{
    BabataResult,
    agent::{Agent, AgentTask},
    config::{AgentConfig, CodexAgentConfig, Config},
    error::BabataError,
    message::Content,
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
            .arg("never");

        if let Some(model) = &self.model {
            command.arg("--model").arg(model);
        }

        command
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

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

        let output = child.wait_with_output().await.map_err(|err| {
            BabataError::internal(format!("Failed to wait for Codex CLI process: {}", err))
        })?;

        if output.status.success() {
            return Ok(());
        }

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let mut details = Vec::new();
        if let Some(code) = output.status.code() {
            details.push(format!("exit code {code}"));
        } else {
            details.push("terminated by signal".to_string());
        }
        if !stdout.is_empty() {
            details.push(format!("stdout: {stdout}"));
        }
        if !stderr.is_empty() {
            details.push(format!("stderr: {stderr}"));
        }

        Err(BabataError::internal(format!(
            "Codex CLI task execution failed ({})",
            details.join(", ")
        )))
    }
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
