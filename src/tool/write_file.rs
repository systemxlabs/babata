use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    BabataResult,
    error::BabataError,
    tool::{Tool, ToolContext, ToolSpec, parse_tool_args},
};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct WriteFileTool {
    spec: ToolSpec,
}

impl Default for WriteFileTool {
    fn default() -> Self {
        Self::new()
    }
}

impl WriteFileTool {
    pub fn new() -> Self {
        Self {
            spec: ToolSpec {
                name: "write_file".to_string(),
                description: "Create a new file or completely overwrite an existing one. Creates parent directories if they don't exist.".to_string(),
                parameters: schemars::schema_for!(WriteFileArgs),
            },
        }
    }
}

#[async_trait::async_trait]
impl Tool for WriteFileTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, context: &ToolContext<'_>) -> BabataResult<String> {
        let args: WriteFileArgs = parse_tool_args(args)?;
        let path = shellexpand::tilde(&args.path).to_string();

        crate::task_info!(context.task_id, "Writing to file: {}", path);

        let file_path = Path::new(&path);

        // Create parent directories if they don't exist
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| BabataError::tool(format!("Failed to create directories: {}", e)))?;
        }

        let lines = args.content.lines().count();

        // Write content to file
        tokio::fs::write(&path, args.content)
            .await
            .map_err(|e| BabataError::tool(format!("Failed to write file: {}", e)))?;

        Ok(format!("Successfully wrote {lines} lines to {path}"))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct WriteFileArgs {
    #[schemars(description = "The path to the file to write")]
    path: String,
    #[schemars(description = "The content to write to the file")]
    content: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn write_file_basic() {
        let tool = WriteFileTool::new();
        let tool_context = ToolContext::test();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("babata_test_write.txt");

        // Clean up before test
        let _ = std::fs::remove_file(&test_file);

        let args = json!({
            "path": test_file.to_str().unwrap(),
            "content": "Hello, Babata!"
        });
        let args = args.to_string();

        let result = tool.execute(&args, &tool_context).await;
        assert!(result.is_ok(), "Write operation should succeed");

        // Verify file was created
        let content = std::fs::read_to_string(&test_file).expect("File should exist");
        assert_eq!(content, "Hello, Babata!");

        // Clean up
        let _ = std::fs::remove_file(&test_file);
    }

    #[tokio::test]
    async fn write_file_creates_directories() {
        let tool = WriteFileTool::new();
        let tool_context = ToolContext::test();
        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join("babata_test_dir");
        let test_subdir = test_dir.join("subdir");
        let test_file = test_subdir.join("test.txt");

        // Clean up before test
        let _ = std::fs::remove_dir_all(&test_dir);

        let args = json!({
            "path": test_file.to_str().unwrap(),
            "content": "Test content"
        });
        let args = args.to_string();

        let result = tool.execute(&args, &tool_context).await;
        assert!(result.is_ok(), "Write operation should create directories");

        // Verify directories and file were created
        assert!(test_dir.exists(), "Parent directory should exist");
        assert!(test_subdir.exists(), "Subdirectory should exist");
        assert!(test_file.exists(), "File should exist");

        let content = std::fs::read_to_string(&test_file).expect("File should be readable");
        assert_eq!(content, "Test content");

        // Clean up
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[tokio::test]
    async fn write_file_missing_path() {
        let tool = WriteFileTool::new();
        let tool_context = ToolContext::test();
        let args = json!({
            "content": "Some content"
        });
        let args = args.to_string();

        let result = tool.execute(&args, &tool_context).await;
        assert!(result.is_err(), "Should fail when path is missing");
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid tool arguments: missing field `path`")
        );
    }

    #[tokio::test]
    async fn write_file_missing_content() {
        let tool = WriteFileTool::new();
        let tool_context = ToolContext::test();
        let args = json!({
            "path": "/tmp/test.txt"
        });
        let args = args.to_string();

        let result = tool.execute(&args, &tool_context).await;
        assert!(result.is_err(), "Should fail when content is missing");
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid tool arguments: missing field `content`")
        );
    }
}
