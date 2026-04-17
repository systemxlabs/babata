use schemars::JsonSchema;
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, BufReader};

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

        read_file_excerpt(&path, offset, limit).await
    }
}

async fn read_file_excerpt(path: &str, offset: usize, limit: usize) -> BabataResult<String> {
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|e| BabataError::tool(format!("Failed to read file: {}", e)))?;
    let mut lines = BufReader::new(file).lines();

    if limit == 0 {
        return Ok(format!(
            "No lines returned from file '{}': limit is 0.",
            path
        ));
    }

    let mut current_line_index = 0usize;
    let mut selected = Vec::new();

    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|e| BabataError::tool(format!("Failed to read file: {}", e)))?
    {
        if current_line_index >= offset && selected.len() < limit {
            selected.push(format!("{:4}\t{}", current_line_index + 1, line));
        }

        current_line_index += 1;

        if selected.len() >= limit {
            break;
        }
    }

    if selected.is_empty() {
        if current_line_index == 0 {
            return Ok(format!("File '{}' is empty.", path));
        }

        if offset >= current_line_index {
            return Ok(format!(
                "No lines found in file '{}': requested offset {} but file has {} line(s).",
                path, offset, current_line_index
            ));
        }
    }

    Ok(selected.join("\n"))
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

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use uuid::Uuid;

    use super::read_file_excerpt;

    fn temp_file_path() -> PathBuf {
        std::env::temp_dir().join(format!("babata-read-file-test-{}.txt", Uuid::new_v4()))
    }

    #[tokio::test]
    async fn read_file_excerpt_returns_message_for_empty_file() {
        let path = temp_file_path();
        fs::write(&path, "").expect("write empty file");

        let output = read_file_excerpt(path.to_str().expect("utf8 path"), 0, 200)
            .await
            .expect("read file");

        assert!(output.contains("is empty"));

        fs::remove_file(path).expect("remove temp file");
    }

    #[tokio::test]
    async fn read_file_excerpt_returns_message_for_offset_past_end() {
        let path = temp_file_path();
        fs::write(&path, "line 1\nline 2\n").expect("write file");

        let output = read_file_excerpt(path.to_str().expect("utf8 path"), 10, 200)
            .await
            .expect("read file");

        assert!(output.contains("requested offset 10"));
        assert!(output.contains("file has 2 line(s)"));

        fs::remove_file(path).expect("remove temp file");
    }

    #[tokio::test]
    async fn read_file_excerpt_reads_requested_line_range() {
        let path = temp_file_path();
        fs::write(&path, "line 1\nline 2\nline 3\n").expect("write file");

        let output = read_file_excerpt(path.to_str().expect("utf8 path"), 1, 1)
            .await
            .expect("read file");

        assert_eq!(output, "   2\tline 2");

        fs::remove_file(path).expect("remove temp file");
    }
}
