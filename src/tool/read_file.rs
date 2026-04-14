use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    BabataResult,
    error::BabataError,
    tool::{Tool, ToolContext, ToolSpec, parse_tool_args},
};

const DEFAULT_MAX_LINES: usize = 2000;

#[derive(Debug, Clone)]
pub struct ReadFileTool {
    spec: ToolSpec,
}

impl Default for ReadFileTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadFileTool {
    pub fn new() -> Self {
        Self {
            spec: ToolSpec {
                name: "read_file".to_string(),
                description: "Read the contents of a file at given path".to_string(),
                parameters: schemars::schema_for!(ReadFileArgs),
            },
        }
    }
}

#[async_trait::async_trait]
impl Tool for ReadFileTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, context: &ToolContext<'_>) -> BabataResult<String> {
        let args: ReadFileArgs = parse_tool_args(args)?;
        let path = shellexpand::tilde(&args.path).to_string();

        let offset = args.offset.unwrap_or(0);
        let limit = args.limit.unwrap_or(DEFAULT_MAX_LINES);

        crate::task_info!(context.task_id, "Reading file: {}", path);

        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| BabataError::tool(format!("Failed to read file: {}", e)))?;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        let start = offset.min(total_lines);
        let end = (start + limit).min(total_lines);

        let selected: Vec<String> = lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{:4}\t{}", start + i + 1, line))
            .collect();

        Ok(selected.join("\n"))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ReadFileArgs {
    #[schemars(description = "The path to the file to read")]
    path: String,
    #[schemars(description = "Line number to start reading from (0-indexed)")]
    offset: Option<usize>,
    #[schemars(description = "Maximum number of lines to read")]
    limit: Option<usize>,
}
