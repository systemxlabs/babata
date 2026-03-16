use log::info;
use serde_json::{Value, json};

use crate::{
    BabataResult,
    agent::babata::tool::{Tool, ToolContext, ToolSpec},
    error::BabataError,
};

#[derive(Debug, Clone)]
pub struct EditFileTool {
    spec: ToolSpec,
}

impl Default for EditFileTool {
    fn default() -> Self {
        Self::new()
    }
}

impl EditFileTool {
    pub fn new() -> Self {
        Self {
            spec: ToolSpec {
                name: "edit_file".to_string(),
                description:
                    "Edit a file by replacing old_string with new_string. Supports single replace or replace_all."
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The path to the file to edit"
                        },
                        "old_string": {
                            "type": "string",
                            "description": "The text to replace"
                        },
                        "new_string": {
                            "type": "string",
                            "description": "Replacement text"
                        },
                        "replace_all": {
                            "type": "boolean",
                            "description": "Whether to replace all occurrences (default: false)"
                        }
                    },
                    "required": ["path", "old_string", "new_string"]
                }),
            },
        }
    }
}

#[async_trait::async_trait]
impl Tool for EditFileTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, _context: &ToolContext) -> BabataResult<String> {
        let args: Value = serde_json::from_str(args)?;
        let path = args["path"]
            .as_str()
            .ok_or_else(|| BabataError::tool("Missing required parameter: path"))?;
        let old_string = args["old_string"]
            .as_str()
            .ok_or_else(|| BabataError::tool("Missing required parameter: old_string"))?;
        let new_string = args["new_string"]
            .as_str()
            .ok_or_else(|| BabataError::tool("Missing required parameter: new_string"))?;
        let replace_all = args["replace_all"].as_bool().unwrap_or(false);

        if old_string.is_empty() {
            return Err(BabataError::tool(
                "old_string cannot be empty for edit_file replacements",
            ));
        }

        let path = shellexpand::tilde(path).to_string();
        info!("Editing file: {}", path);

        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|err| BabataError::tool(format!("Failed to read file: {}", err)))?;

        let (new_content, count) = if replace_all {
            let count = content.matches(old_string).count();
            (content.replace(old_string, new_string), count)
        } else if content.contains(old_string) {
            (content.replacen(old_string, new_string, 1), 1)
        } else {
            return Err(BabataError::tool(
                "old_string not found in file; no changes were made",
            ));
        };

        tokio::fs::write(&path, new_content)
            .await
            .map_err(|err| BabataError::tool(format!("Failed to write file: {}", err)))?;

        Ok(format!("Replaced {} occurrence(s) in {}", count, path))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use uuid::Uuid;

    use super::*;

    fn temp_file_path(prefix: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("{prefix}-{}.txt", Uuid::new_v4()))
    }

    #[tokio::test]
    async fn edit_file_replaces_first_match_by_default() {
        let tool = EditFileTool::new();
        let tool_context = ToolContext::test();
        let file = temp_file_path("babata-edit-first");
        tokio::fs::write(&file, "hello world\nhello world")
            .await
            .expect("seed file");

        let args = json!({
            "path": file.to_string_lossy(),
            "old_string": "hello",
            "new_string": "hi"
        })
        .to_string();

        let result = tool.execute(&args, &tool_context).await.expect("edit file");
        assert!(result.contains("Replaced 1 occurrence(s)"));

        let content = tokio::fs::read_to_string(&file).await.expect("read file");
        assert_eq!(content, "hi world\nhello world");

        let _ = tokio::fs::remove_file(&file).await;
    }

    #[tokio::test]
    async fn edit_file_replace_all_replaces_all_matches() {
        let tool = EditFileTool::new();
        let tool_context = ToolContext::test();
        let file = temp_file_path("babata-edit-all");
        tokio::fs::write(&file, "alpha beta alpha beta")
            .await
            .expect("seed file");

        let args = json!({
            "path": file.to_string_lossy(),
            "old_string": "alpha",
            "new_string": "A",
            "replace_all": true
        })
        .to_string();

        let result = tool.execute(&args, &tool_context).await.expect("edit file");
        assert!(result.contains("Replaced 2 occurrence(s)"));

        let content = tokio::fs::read_to_string(&file).await.expect("read file");
        assert_eq!(content, "A beta A beta");

        let _ = tokio::fs::remove_file(&file).await;
    }

    #[tokio::test]
    async fn edit_file_fails_when_old_string_not_found() {
        let tool = EditFileTool::new();
        let tool_context = ToolContext::test();
        let file = temp_file_path("babata-edit-not-found");
        tokio::fs::write(&file, "content").await.expect("seed file");

        let args = json!({
            "path": file.to_string_lossy(),
            "old_string": "missing",
            "new_string": "value"
        })
        .to_string();

        let err = tool
            .execute(&args, &tool_context)
            .await
            .expect_err("should fail");
        assert!(
            err.to_string()
                .contains("old_string not found in file; no changes were made")
        );

        let _ = tokio::fs::remove_file(&file).await;
    }

    #[tokio::test]
    async fn edit_file_fails_when_old_string_is_empty() {
        let tool = EditFileTool::new();
        let tool_context = ToolContext::test();
        let file = temp_file_path("babata-edit-empty-old");
        tokio::fs::write(&file, "content").await.expect("seed file");

        let args = json!({
            "path": file.to_string_lossy(),
            "old_string": "",
            "new_string": "value"
        })
        .to_string();

        let err = tool
            .execute(&args, &tool_context)
            .await
            .expect_err("should fail");
        assert!(
            err.to_string()
                .contains("old_string cannot be empty for edit_file replacements")
        );

        let _ = tokio::fs::remove_file(&file).await;
    }
}
