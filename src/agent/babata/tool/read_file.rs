use serde_json::{Value, json};

use crate::{
    BabataResult,
    agent::babata::{Tool, ToolContext, ToolSpec},
    error::BabataError,
};
use log::info;

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
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The path to the file to read"
                        },
                        "offset": {
                            "type": "integer",
                            "description": "Line number to start reading from (0-indexed)"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of lines to read"
                        }
                    },
                    "required": ["path"]
                }),
            },
        }
    }
}

#[async_trait::async_trait]
impl Tool for ReadFileTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, _context: &ToolContext<'_>) -> BabataResult<String> {
        let (path, offset, limit) = parse_args(args)?;

        info!("Reading file: {}", path);

        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| BabataError::tool(format!("Failed to read file: {}", e)))?;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        let start = offset.min(total_lines);
        let end = limit
            .map(|l| (start + l).min(total_lines))
            .unwrap_or(total_lines);

        let selected: Vec<String> = lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{:4}\t{}", start + i + 1, line))
            .collect();

        Ok(selected.join("\n"))
    }
}

fn parse_args(args: &str) -> BabataResult<(String, usize, Option<usize>)> {
    let args: Value = serde_json::from_str(args)?;
    let path = args["path"]
        .as_str()
        .ok_or_else(|| BabataError::tool("Missing path"))?;

    let path = shellexpand::tilde(path).to_string();
    let offset = args["offset"].as_u64().unwrap_or(0) as usize;
    let limit = args["limit"].as_u64().map(|l| l as usize);

    Ok((path, offset, limit))
}
