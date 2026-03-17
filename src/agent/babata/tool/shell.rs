use log::info;
use serde_json::{Value, json};

use crate::{
    BabataResult,
    agent::babata::{Tool, ToolContext, ToolSpec},
    error::BabataError,
};

#[derive(Debug)]
pub struct ShellTool {
    spec: ToolSpec,
    default_timeout_ms: u64,
}

impl ShellTool {
    pub fn new() -> Self {
        let default_timeout_ms = 30000;
        let spec = ToolSpec {
            name: "shell".to_string(),
            description:
                "Execute a shell command and return the output. Uses bash on Linux/macOS and PowerShell on Windows."
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute (bash syntax on Linux/macOS, PowerShell syntax on Windows)"
                    },
                    "timeout_ms": {
                        "type": "integer",
                        "description": format!("Optional timeout in milliseconds (default: {default_timeout_ms})")
                    }
                },
                "required": ["command"]
            }),
        };
        Self {
            spec,
            default_timeout_ms,
        }
    }
}

#[async_trait::async_trait]
impl Tool for ShellTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, _context: &ToolContext<'_>) -> BabataResult<String> {
        info!("Executing shell command: {args}",);

        let args: Value = serde_json::from_str(args)?;
        let command = args["command"]
            .as_str()
            .ok_or_else(|| BabataError::tool("Missing command"))?;

        let timeout_ms = args["timeout_ms"]
            .as_u64()
            .unwrap_or(self.default_timeout_ms);

        // Run command with timeout
        let timeout_duration = std::time::Duration::from_millis(timeout_ms);
        let output = tokio::time::timeout(timeout_duration, spawn_shell_command(command))
            .await
            .map_err(|_| BabataError::tool(format!("Command timed out after {}ms", timeout_ms)))?
            .map_err(|e| BabataError::tool(format!("Failed to execute command: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let mut result = String::new();

        if !stdout.is_empty() {
            result.push_str(&stdout);
        }

        if !stderr.is_empty() {
            if !result.is_empty() {
                result.push_str("\n\nSTDERR:\n");
            }
            result.push_str(&stderr);
        }

        if result.is_empty() {
            result = format!(
                "Command completed with exit code: {}",
                output.status.code().unwrap_or(-1)
            );
        }

        Ok(result)
    }
}

impl Default for ShellTool {
    fn default() -> Self {
        Self::new()
    }
}

fn spawn_shell_command(
    command: &str,
) -> impl std::future::Future<Output = std::io::Result<std::process::Output>> {
    let mut process = match std::env::consts::OS {
        "windows" => {
            let mut cmd = tokio::process::Command::new("powershell.exe");
            let wrapped_command = build_windows_command(command);
            cmd.arg("-NoProfile")
                .arg("-NonInteractive")
                .arg("-ExecutionPolicy")
                .arg("Bypass")
                .arg("-Command")
                .arg(wrapped_command);
            cmd
        }
        _ => {
            let mut cmd = tokio::process::Command::new("bash");
            cmd.arg("-c").arg(command);
            cmd
        }
    };

    process.output()
}

fn build_windows_command(command: &str) -> String {
    let utf8_session_setup = r#"$utf8NoBom = [System.Text.UTF8Encoding]::new($false);
$OutputEncoding = $utf8NoBom;
[Console]::InputEncoding = $utf8NoBom;
[Console]::OutputEncoding = $utf8NoBom;
$PSDefaultParameterValues['Out-File:Encoding'] = 'utf8';
$PSDefaultParameterValues['Set-Content:Encoding'] = 'utf8';
$PSDefaultParameterValues['Add-Content:Encoding'] = 'utf8';"#;
    format!("{utf8_session_setup}\n{command}")
}

#[cfg(test)]
mod tests {
    use super::build_windows_command;

    #[test]
    fn build_windows_command_includes_utf8_setup_and_original_command() {
        let wrapped = build_windows_command("Write-Output 'hello'");
        assert!(wrapped.contains("$OutputEncoding"));
        assert!(wrapped.contains("Set-Content:Encoding"));
        assert!(wrapped.contains("Write-Output 'hello'"));
    }
}
