use std::{path::PathBuf, process::Output, time::Duration};

use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    BabataResult,
    error::BabataError,
    tool::{Tool, ToolContext, ToolSpec, parse_tool_args},
    utils::task_dir,
};

const DEFAULT_TIMEOUT_SECS: usize = 300;
const DEFAULT_MAX_LINES: usize = 2000;

#[derive(Debug)]
pub struct ShellTool {
    spec: ToolSpec,
}

impl ShellTool {
    pub fn new() -> Self {
        let spec = ToolSpec {
            name: "shell".to_string(),
            description: format!(
                "Execute a shell command and return stdout and stderr. Output is truncated to last {DEFAULT_MAX_LINES} lines. If truncated, full output is saved to a temp file."
            ),
            parameters: schemars::schema_for!(ShellArgs),
        };
        Self { spec }
    }
}

#[async_trait::async_trait]
impl Tool for ShellTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, context: &ToolContext<'_>) -> BabataResult<String> {
        crate::task_info!(context.task_id, "Executing shell command: {args}");

        let args: ShellArgs = parse_tool_args(args)?;

        let timeout_secs = args.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS);

        let output = exec_shell(&args.command, timeout_secs).await?;

        let stdout = process_output_with_truncation(&output.stdout, context, "stdout")?;
        let stderr = process_output_with_truncation(&output.stderr, context, "stderr")?;
        let exit_status = output.status.code().unwrap_or(-1);

        let result = format!(
            r#"# STDOUT

{}

# STDERR

{}

# Exit Status

{exit_status}"#,
            if stdout.is_empty() {
                "(no output)".to_string()
            } else {
                stdout
            },
            if stderr.is_empty() {
                "(no output)".to_string()
            } else {
                stderr
            },
        );

        Ok(result)
    }
}

impl Default for ShellTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ShellArgs {
    #[schemars(
        description = "The shell command to execute (bash syntax on Linux/macOS, PowerShell syntax on Windows)"
    )]
    command: String,
    #[schemars(description = "Optional timeout in seconds")]
    timeout_secs: Option<usize>,
}

async fn exec_shell(command: &str, timeout_secs: usize) -> BabataResult<Output> {
    // Run command with timeout
    let mut shell_cmd = create_command(command);
    let output = tokio::time::timeout(Duration::from_secs(timeout_secs as u64), shell_cmd.output())
        .await
        .map_err(|_| BabataError::tool(format!("Command timed out after {}s", timeout_secs)))?
        .map_err(|e| BabataError::tool(format!("Failed to execute command: {}", e)))?;

    Ok(output)
}

pub fn detect_shell_type() -> &'static str {
    match std::env::consts::OS {
        "windows" => "powershell.exe",
        _ => "bash",
    }
}

fn create_command(command: &str) -> tokio::process::Command {
    let mut shell_cmd = tokio::process::Command::new(detect_shell_type());
    match std::env::consts::OS {
        "windows" => {
            let utf8_session_setup = r#"$utf8NoBom = [System.Text.UTF8Encoding]::new($false);
$OutputEncoding = $utf8NoBom;
[Console]::InputEncoding = $utf8NoBom;
[Console]::OutputEncoding = $utf8NoBom;
$PSDefaultParameterValues['Out-File:Encoding'] = 'utf8';
$PSDefaultParameterValues['Set-Content:Encoding'] = 'utf8';
$PSDefaultParameterValues['Add-Content:Encoding'] = 'utf8';"#;
            let wrapped_command = format!("{utf8_session_setup}\n{command}");
            shell_cmd
                .arg("-NoProfile")
                .arg("-NonInteractive")
                .arg("-ExecutionPolicy")
                .arg("Bypass")
                .arg("-Command")
                .arg(wrapped_command);
            shell_cmd
        }
        _ => {
            shell_cmd.arg("-c").arg(command);
            shell_cmd
        }
    }
}

fn process_output_with_truncation(
    output: &[u8],
    context: &ToolContext<'_>,
    stream_name: &str,
) -> BabataResult<String> {
    let output = String::from_utf8_lossy(output);
    let lines: Vec<&str> = output.lines().collect();

    if lines.len() <= DEFAULT_MAX_LINES {
        return Ok(output.to_string());
    }

    // Truncate: keep only the last max_lines lines
    let truncated_lines = &lines[lines.len() - DEFAULT_MAX_LINES..];
    let truncated_output = truncated_lines.join("\n");

    // Write full output to file
    let log_file_path = get_shell_log_path(context, stream_name)?;
    std::fs::write(&log_file_path, output.as_ref()).map_err(|e| {
        BabataError::internal(format!(
            "Failed to write shell {} log to '{}': {}",
            stream_name,
            log_file_path.display(),
            e
        ))
    })?;

    let truncated_header = format!(
        "... (output truncated, showing last {} lines, full output written to {})\n",
        DEFAULT_MAX_LINES,
        log_file_path.display()
    );

    Ok(truncated_header + &truncated_output)
}

fn get_shell_log_path(context: &ToolContext<'_>, stream_name: &str) -> BabataResult<PathBuf> {
    let task_dir = task_dir(*context.task_id)?;
    let log_file_name = format!("shell-call-{}-{}.log", context.call_id, stream_name);
    Ok(task_dir.join(log_file_name))
}

#[cfg(test)]
mod tests {
    use super::{create_command, detect_shell_type};

    #[test]
    fn spawn_shell_command_windows_includes_utf8_setup() {
        if std::env::consts::OS != "windows" {
            return;
        }
        let cmd = create_command("Write-Output 'hello'");
        let args: Vec<&std::ffi::OsStr> = cmd.as_std().get_args().collect();
        let command_arg = args.last().unwrap().to_str().unwrap();
        assert!(command_arg.contains("$OutputEncoding"));
        assert!(command_arg.contains("Set-Content:Encoding"));
        assert!(command_arg.contains("Write-Output 'hello'"));
    }

    #[test]
    fn spawn_shell_command_unix_uses_bash() {
        if std::env::consts::OS == "windows" {
            return;
        }
        let cmd = create_command("echo hello");
        assert_eq!(cmd.as_std().get_program(), "bash");
    }

    #[test]
    fn detect_shell_type_matches_platform() {
        let shell = detect_shell_type();
        if std::env::consts::OS == "windows" {
            assert_eq!(shell, "powershell");
        } else {
            assert_eq!(shell, "bash");
        }
    }
}
