use log::debug;
use serde_json::{Value, json};

use crate::{
    BabataResult,
    error::BabataError,
    tool::{Tool, ToolSpec},
};

#[derive(Debug)]
pub struct BashTool {
    spec: ToolSpec,
    default_timeout_ms: u64,
}

impl BashTool {
    pub fn new() -> Self {
        let default_timeout_ms = 30000;
        let spec = ToolSpec {
            name: "bash".to_string(),
            description: "Execute a bash command and return the output".to_string(),
            parameters: json!({

            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The bash command to execute"
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
impl Tool for BashTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: Value) -> BabataResult<String> {
        debug!("Executing bash command: {args}",);

        let command = args["command"]
            .as_str()
            .ok_or_else(|| BabataError::tool("Missing command"))?;

        let timeout_ms = args["timeout_ms"]
            .as_u64()
            .unwrap_or(self.default_timeout_ms);

        // Run command with timeout
        let timeout_duration = std::time::Duration::from_millis(timeout_ms);
        let output = tokio::time::timeout(
            timeout_duration,
            tokio::process::Command::new("bash")
                .arg("-c")
                .arg(command)
                .output(),
        )
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

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}
