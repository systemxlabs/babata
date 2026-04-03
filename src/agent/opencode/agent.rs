use std::{
    fs::File,
    path::{Path, PathBuf},
    process::Stdio,
};

use log::info;
use tokio::process::Command;
use uuid::Uuid;

use crate::{
    BabataResult,
    agent::{
        Agent, AgentTask,
        prompt::{
            BABATA_SYSTEM_DESCRIPTION, build_agents_prompt, build_channels_prompt,
            build_runtime_prompt, build_skills_prompt, load_workspace_prompt,
        },
        skill,
    },
    config::{AgentConfig, Config, OpencodeAgentConfig},
    error::BabataError,
    message::Content,
    task::task_dir,
};

/// Environment variable for OpenCode configuration to enable YOLO mode
/// This disables interactive prompts and allows automatic execution
const OPENCODE_CONFIG_CONTENT: &str = r#"{
    "permission": {
        "*": "allow",
        "external_directory": "allow",
        "question": "deny"
    }
}"#;

#[derive(Debug, Clone)]
pub struct OpencodeAgent {
    command: String,
    workspace: PathBuf,
    model: Option<String>,
}

impl OpencodeAgent {
    pub fn new(config: &Config) -> BabataResult<Self> {
        let agent_config = config.get_agent(OpencodeAgent::name())?;
        let AgentConfig::Opencode(opencode_config) = agent_config else {
            return Err(BabataError::config(
                "Agent config for 'opencode' must be of type 'OpencodeAgentConfig'",
            ));
        };

        Ok(Self::from_config(opencode_config))
    }

    fn from_config(config: &OpencodeAgentConfig) -> Self {
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

        // Load config from disk on each execution
        let config = Config::load()?;
        let runtime_prompt = build_runtime_prompt()?;
        let agents_prompt = build_agents_prompt(&config);
        let channels_prompt = build_channels_prompt(&config)?;
        let skills = skill::load_skills()?;
        let skills_prompt = build_skills_prompt(&skills).unwrap_or_default();
        let workspace_prompt = load_workspace_prompt()?.unwrap_or_default();

        Ok(format!(
            r#"{}

{}

{}

{}

{}

{}

Task metadata:
- task_id: {}
- parent_task_id: {}
- root_task_id: {}

User task:
{}
"#,
            BABATA_SYSTEM_DESCRIPTION,
            runtime_prompt,
            agents_prompt,
            channels_prompt,
            skills_prompt,
            workspace_prompt,
            task.task_id,
            parent_task_id,
            task.root_task_id,
            prompt_text
        ))
    }

    fn output_paths(&self, task_id: Uuid) -> BabataResult<OpencodeOutputPaths> {
        let task_dir = task_dir(task_id)?;
        Ok(OpencodeOutputPaths {
            stdout: task_dir.join("opencode-stdout.log"),
            stderr: task_dir.join("opencode-stderr.log"),
        })
    }
}

#[async_trait::async_trait]
impl Agent for OpencodeAgent {
    fn name() -> &'static str {
        "opencode"
    }

    fn description() -> &'static str {
        "Use for code writing and code review tasks"
    }

    async fn execute(&self, task: AgentTask) -> BabataResult<()> {
        let prompt = self.build_prompt(&task)?;
        let output_paths = self.output_paths(task.task_id)?;
        info!(
            "Executing task {} with OpenCode CLI in workspace {}",
            task.task_id,
            self.workspace.display()
        );

        let mut command = Command::new(&self.command);
        command
            .arg("run")
            .arg(&prompt)
            .arg("--continue")
            .current_dir(&self.workspace)
            .kill_on_drop(true);

        if let Some(model) = &self.model {
            command.arg("--model").arg(model);
        }

        // Set environment variable to enable YOLO mode (auto-approve)
        // This prevents opencode from hanging on interactive prompts
        command.env("OPENCODE_CONFIG_CONTENT", OPENCODE_CONFIG_CONTENT);

        let stdout = File::create(&output_paths.stdout).map_err(|err| {
            BabataError::internal(format!(
                "Failed to create OpenCode stdout log '{}': {}",
                output_paths.stdout.display(),
                err
            ))
        })?;
        let stderr = File::create(&output_paths.stderr).map_err(|err| {
            BabataError::internal(format!(
                "Failed to create OpenCode stderr log '{}': {}",
                output_paths.stderr.display(),
                err
            ))
        })?;

        command
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr));

        let status = command.status().await.map_err(|err| {
            BabataError::internal(format!(
                "Failed to start or wait for OpenCode CLI '{}': {}",
                self.command, err
            ))
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
        let stdout_log = read_trimmed_file(&output_paths.stdout)?;
        let stderr_log = read_trimmed_file(&output_paths.stderr)?;
        if !stdout_log.is_empty() {
            details.push(format!("stdout log: {stdout_log}"));
        }
        if !stderr_log.is_empty() {
            details.push(format!("stderr log: {stderr_log}"));
        }

        Err(BabataError::internal(format!(
            "OpenCode CLI task execution failed ({})",
            details.join(", ")
        )))
    }
}

#[derive(Debug)]
struct OpencodeOutputPaths {
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
                    "Opencode agent only supports text prompt content",
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
    #[ignore = "requires babata home directory with config.json"]
    fn build_prompt_includes_task_metadata_and_workspace() {
        let agent = OpencodeAgent {
            command: "opencode".to_string(),
            workspace: PathBuf::from("C:/workspace/project"),
            model: Some("claude-sonnet-4".to_string()),
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
