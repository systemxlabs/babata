use log::info;
use schemars::JsonSchema;
use serde::Deserialize;
use similar::{ChangeTag, TextDiff};

use crate::{
    BabataResult,
    error::BabataError,
    tool::{Tool, ToolContext, ToolSpec, parse_tool_args},
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
                    "Edit a file by replacing an exact string match. old_string must appear exactly once in the file for safety. Include enough surrounding context to ensure uniqueness."
                        .to_string(),
                parameters: schemars::schema_for!(EditFileArgs),
            },
        }
    }
}

#[async_trait::async_trait]
impl Tool for EditFileTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, _context: &ToolContext<'_>) -> BabataResult<String> {
        let (file_path, old_string, new_string) = validate_args(args)?;

        info!("Editing file: {}", file_path);

        let content = tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|err| BabataError::tool(format!("Failed to read file: {}", err)))?;

        let occurrences = content.matches(&old_string).count();

        if occurrences == 0 {
            let preview = if content.len() > 500 {
                format!("{}...", &content[..500])
            } else {
                content.clone()
            };
            return Err(BabataError::tool(format!(
                "old_string not found in {}.\nFile starts with:\n{}",
                file_path, preview
            )));
        }

        if occurrences > 1 {
            return Err(BabataError::tool(format!(
                "old_string appears {} times in {}. Include more surrounding lines to make it unique.",
                occurrences, file_path
            )));
        }

        let new_content = content.replacen(&old_string, &new_string, 1);

        tokio::fs::write(&file_path, &new_content)
            .await
            .map_err(|err| BabataError::tool(format!("Failed to write file: {}", err)))?;

        // generate a unified diff
        let diff = unified_diff(&content, &new_content, &file_path, 3);

        Ok(format!("Edited {}\n{}", file_path, diff))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct EditFileArgs {
    #[schemars(description = "Path to the file to edit")]
    file_path: String,
    #[schemars(description = "Exact text to find (must be unique in file)")]
    old_string: String,
    #[schemars(description = "Replacement text")]
    new_string: String,
}

fn validate_args(args: &str) -> BabataResult<(String, String, String)> {
    let args: EditFileArgs = parse_tool_args(args)?;
    Ok((
        shellexpand::tilde(&args.file_path).to_string(),
        args.old_string,
        args.new_string,
    ))
}

/// Generate a compact unified diff between old and new file content
fn unified_diff(old: &str, new: &str, filename: &str, context: usize) -> String {
    let diff = TextDiff::from_lines(old, new);
    let mut result = String::new();

    result.push_str(&format!("--- a/{filename}\n"));
    result.push_str(&format!("+++ b/{filename}\n"));

    for group in diff.grouped_ops(context) {
        // Find the range for this hunk
        let mut old_start = usize::MAX;
        let mut old_len = 0;
        let mut new_start = usize::MAX;
        let mut new_len = 0;

        for op in &group {
            let (_tag, old_range, new_range) = op.as_tag_tuple();
            old_start = old_start.min(old_range.start);
            old_len += old_range.len();
            new_start = new_start.min(new_range.start);
            new_len += new_range.len();
        }

        result.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            old_start + 1,
            old_len,
            new_start + 1,
            new_len
        ));

        for op in group {
            for change in diff.iter_changes(&op) {
                let sign = match change.tag() {
                    ChangeTag::Delete => '-',
                    ChangeTag::Insert => '+',
                    ChangeTag::Equal => ' ',
                };
                result.push_str(&format!("{}{}", sign, change.value()));
            }
        }
    }

    // truncate enormous diffs
    if result.len() > 3000 {
        format!("{}\n... (diff truncated)\n", &result[..2500])
    } else {
        result
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
    async fn edit_file_replaces_unique_match() {
        let tool = EditFileTool::new();
        let tool_context = ToolContext::test();
        let file = temp_file_path("babata-edit-unique");
        tokio::fs::write(&file, "hello world\ngoodbye world")
            .await
            .expect("seed file");

        let args = json!({
            "file_path": file.to_string_lossy(),
            "old_string": "hello",
            "new_string": "hi"
        })
        .to_string();

        let result = tool.execute(&args, &tool_context).await.expect("edit file");
        assert!(result.contains("Edited"));
        assert!(result.contains("-hello"));
        assert!(result.contains("+hi"));

        let content = tokio::fs::read_to_string(&file).await.expect("read file");
        assert_eq!(content, "hi world\ngoodbye world");

        let _ = tokio::fs::remove_file(&file).await;
    }

    #[tokio::test]
    async fn edit_file_fails_when_old_string_not_found() {
        let tool = EditFileTool::new();
        let tool_context = ToolContext::test();
        let file = temp_file_path("babata-edit-not-found");
        tokio::fs::write(&file, "content here")
            .await
            .expect("seed file");

        let args = json!({
            "file_path": file.to_string_lossy(),
            "old_string": "missing",
            "new_string": "value"
        })
        .to_string();

        let err = tool
            .execute(&args, &tool_context)
            .await
            .expect_err("should fail");
        let err_msg = err.to_string();
        assert!(err_msg.contains("old_string not found"));
        assert!(err_msg.contains("File starts with:"));
        assert!(err_msg.contains("content here"));

        let _ = tokio::fs::remove_file(&file).await;
    }

    #[tokio::test]
    async fn edit_file_fails_when_old_string_appears_multiple_times() {
        let tool = EditFileTool::new();
        let tool_context = ToolContext::test();
        let file = temp_file_path("babata-edit-multiple");
        tokio::fs::write(&file, "hello world\nhello again")
            .await
            .expect("seed file");

        let args = json!({
            "file_path": file.to_string_lossy(),
            "old_string": "hello",
            "new_string": "hi"
        })
        .to_string();

        let err = tool
            .execute(&args, &tool_context)
            .await
            .expect_err("should fail");
        assert!(err.to_string().contains("old_string appears 2 times"));
        assert!(
            err.to_string()
                .contains("Include more surrounding lines to make it unique")
        );

        // File should not be modified
        let content = tokio::fs::read_to_string(&file).await.expect("read file");
        assert_eq!(content, "hello world\nhello again");

        let _ = tokio::fs::remove_file(&file).await;
    }

    #[tokio::test]
    async fn edit_file_succeeds_with_more_context_for_duplicate() {
        let tool = EditFileTool::new();
        let tool_context = ToolContext::test();
        let file = temp_file_path("babata-edit-context");
        tokio::fs::write(&file, "hello world\nhello again")
            .await
            .expect("seed file");

        // Use more context to make it unique
        let args = json!({
            "file_path": file.to_string_lossy(),
            "old_string": "hello again",
            "new_string": "hi again"
        })
        .to_string();

        let result = tool.execute(&args, &tool_context).await.expect("edit file");
        assert!(result.contains("Edited"));

        let content = tokio::fs::read_to_string(&file).await.expect("read file");
        assert_eq!(content, "hello world\nhi again");

        let _ = tokio::fs::remove_file(&file).await;
    }

    #[test]
    fn unified_diff_generates_correct_format() {
        let old = "line1\nline2\nline3\n";
        let new = "line1\nmodified\nline3\n";
        let diff = unified_diff(old, new, "test.txt", 3);

        assert!(diff.contains("--- a/test.txt"));
        assert!(diff.contains("+++ b/test.txt"));
        assert!(diff.contains("@@"));
        assert!(diff.contains("-line2"));
        assert!(diff.contains("+modified"));
    }

    #[test]
    fn validate_args_extracts_file_path_and_strings() {
        let (file_path, old_string, new_string) = validate_args(
            &json!({ "file_path": "/tmp/test.txt", "old_string": "foo", "new_string": "bar" })
                .to_string(),
        )
        .expect("parse args");
        assert_eq!(file_path, "/tmp/test.txt");
        assert_eq!(old_string, "foo");
        assert_eq!(new_string, "bar");
    }

    #[test]
    fn validate_args_expands_tilde_in_file_path() {
        let (file_path, _, _) = validate_args(
            &json!({ "file_path": "~/test.txt", "old_string": "a", "new_string": "b" }).to_string(),
        )
        .expect("parse args");
        assert!(!file_path.starts_with('~'));
    }

    #[test]
    fn validate_args_rejects_missing_file_path() {
        let err = validate_args(&json!({ "old_string": "a", "new_string": "b" }).to_string())
            .expect_err("missing file_path");
        assert!(
            err.to_string()
                .contains("Missing required parameter: file_path")
        );
    }

    #[test]
    fn validate_args_rejects_missing_old_string() {
        let err =
            validate_args(&json!({ "file_path": "/tmp/test.txt", "new_string": "b" }).to_string())
                .expect_err("missing old_string");
        assert!(
            err.to_string()
                .contains("Missing required parameter: old_string")
        );
    }

    #[test]
    fn validate_args_rejects_missing_new_string() {
        let err =
            validate_args(&json!({ "file_path": "/tmp/test.txt", "old_string": "a" }).to_string())
                .expect_err("missing new_string");
        assert!(
            err.to_string()
                .contains("Missing required parameter: new_string")
        );
    }
}
