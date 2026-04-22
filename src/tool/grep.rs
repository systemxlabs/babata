use grep::{
    regex::RegexMatcherBuilder,
    searcher::{SearcherBuilder, sinks::UTF8},
};
use schemars::JsonSchema;
use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::{
    BabataResult,
    error::BabataError,
    tool::{Tool, ToolContext, ToolSpec, parse_tool_args, resolve_tool_path},
};

// skip these dirs to avoid noise
const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "__pycache__",
    ".venv",
    "venv",
    ".tox",
    "dist",
    "build",
    "target",
    ".idea",
    ".vscode",
];
const MAX_MATCHES: usize = 200;
const MAX_FILES: usize = 5000;

#[derive(Debug, Clone)]
pub struct GrepTool {
    spec: ToolSpec,
}

impl Default for GrepTool {
    fn default() -> Self {
        Self::new()
    }
}

impl GrepTool {
    pub fn new() -> Self {
        Self {
            spec: ToolSpec {
                name: "grep".to_string(),
                description: format!(
                    "Search file contents with regex. Returns matching lines with file path and line number. Skips directories: {:?}. Returns at most {} matches.",
                    SKIP_DIRS, MAX_MATCHES
                ),
                parameters: schemars::schema_for!(GrepArgs),
            },
        }
    }
}

/// Walk directory tree, skipping junk dirs
fn walk(root: &Path, include: Option<&str>) -> Vec<PathBuf> {
    let pattern = include.unwrap_or("*");
    let mut results = Vec::new();

    for entry in globwalk::GlobWalkerBuilder::new(root, pattern)
        .build()
        .into_iter()
        .flatten()
        .flatten()
    {
        let path = entry.path();

        // skip hidden/junk directories
        if path.components().any(|c| {
            c.as_os_str()
                .to_str()
                .map(|s| SKIP_DIRS.contains(&s))
                .unwrap_or(false)
        }) {
            continue;
        }

        if path.is_file() {
            results.push(path.to_path_buf());
            if results.len() >= MAX_FILES {
                break;
            }
        }
    }

    results
}

#[async_trait::async_trait]
impl Tool for GrepTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, _context: &ToolContext<'_>) -> BabataResult<String> {
        let args: GrepArgs = parse_tool_args(args)?;

        let path = resolve_tool_path(args.path);
        let matcher = RegexMatcherBuilder::new()
            .build(&args.pattern)
            .map_err(|err| BabataError::tool(format!("Invalid regex: {}", err)))?;
        let mut searcher = SearcherBuilder::new().line_number(true).build();

        let base = PathBuf::from(&path);
        if !base.exists() {
            return Err(BabataError::tool(format!("'{}' not found", path)));
        }

        let files: Vec<PathBuf> = if base.is_file() {
            vec![base]
        } else {
            walk(&base, args.include.as_deref())
        };

        let mut matches = Vec::new();

        for fp in files {
            let file_display = fp.display().to_string();
            let result = searcher.search_path(
                &matcher,
                &fp,
                UTF8(|line_num, line| {
                    matches.push(format!(
                        "{}:{}: {}",
                        file_display,
                        line_num,
                        line.trim_end()
                    ));
                    Ok(matches.len() < MAX_MATCHES)
                }),
            );

            if result.is_err() {
                continue;
            }

            if matches.len() >= MAX_MATCHES {
                matches.push("... (match limit reached)".to_string());
                return Ok(matches.join("\n"));
            }
        }

        if matches.is_empty() {
            Ok("No matches found.".to_string())
        } else {
            Ok(matches.join("\n"))
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct GrepArgs {
    #[schemars(description = "Regex pattern to search for")]
    pattern: String,
    #[schemars(description = "File or directory to search (default: current working directory)")]
    path: Option<String>,
    #[schemars(description = "Only search files matching this glob (e.g. '*.py')")]
    include: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{GrepArgs, GrepTool};
    use crate::{
        BabataResult,
        tool::{Tool, ToolContext, parse_tool_args},
    };
    use serde_json::json;
    use uuid::Uuid;

    fn temp_dir_path(prefix: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("{prefix}-{}", Uuid::new_v4()))
    }

    fn parse_grep_args(args: &str) -> BabataResult<(String, String, Option<String>)> {
        let args: GrepArgs = parse_tool_args(args)?;
        let path = super::resolve_tool_path(args.path);
        Ok((args.pattern, path, args.include))
    }

    #[test]
    fn validate_args_extracts_pattern_path_and_include() {
        let (pattern, path, include) = parse_grep_args(
            &json!({ "pattern": "foo", "path": "/tmp", "include": "*.rs" }).to_string(),
        )
        .expect("parse args");
        assert_eq!(pattern, "foo");
        assert_eq!(path, "/tmp");
        assert_eq!(include, Some("*.rs".to_string()));
    }

    #[test]
    fn validate_args_expands_tilde_in_path() {
        let (pattern, path, include) =
            parse_grep_args(&json!({ "pattern": "test", "path": "~" }).to_string())
                .expect("parse args");
        assert_eq!(pattern, "test");
        assert!(!path.starts_with('~'));
        assert_eq!(include, None);
    }

    #[test]
    fn validate_args_uses_cwd_when_path_missing() {
        let (pattern, path, include) =
            parse_grep_args(&json!({ "pattern": "hello" }).to_string()).expect("parse args");
        assert_eq!(pattern, "hello");
        assert!(!path.is_empty());
        assert_eq!(include, None);
    }

    #[test]
    fn validate_args_rejects_missing_pattern() {
        let err =
            parse_grep_args(&json!({ "path": "/tmp" }).to_string()).expect_err("missing pattern");
        assert!(
            err.to_string().contains("Invalid tool arguments")
                && err.to_string().contains("missing field `pattern`")
        );
    }

    #[tokio::test]
    async fn grep_tool_returns_matching_lines_with_file_and_line_number() {
        let tool = GrepTool::new();
        let tool_context = ToolContext::test();
        let dir = temp_dir_path("babata-grep-match");
        let file = dir.join("main.rs");

        tokio::fs::create_dir_all(&dir).await.expect("create dir");
        tokio::fs::write(&file, "fn main() {}\nlet value = 42;\n")
            .await
            .expect("seed file");

        let args = json!({
            "pattern": "value",
            "path": dir.to_string_lossy(),
        })
        .to_string();

        let result = tool.execute(&args, &tool_context).await.expect("grep");
        assert_eq!(result, format!("{}:2: let value = 42;", file.display()));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn grep_tool_respects_include_filter() {
        let tool = GrepTool::new();
        let tool_context = ToolContext::test();
        let dir = temp_dir_path("babata-grep-include");
        let rs_file = dir.join("lib.rs");
        let txt_file = dir.join("notes.txt");

        tokio::fs::create_dir_all(&dir).await.expect("create dir");
        tokio::fs::write(&rs_file, "let rust_match = true;\n")
            .await
            .expect("seed rs file");
        tokio::fs::write(&txt_file, "rust_match should be ignored\n")
            .await
            .expect("seed txt file");

        let args = json!({
            "pattern": "rust_match",
            "path": dir.to_string_lossy(),
            "include": "*.rs",
        })
        .to_string();

        let result = tool.execute(&args, &tool_context).await.expect("grep");
        assert_eq!(
            result,
            format!("{}:1: let rust_match = true;", rs_file.display())
        );

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn grep_tool_returns_no_matches_message() {
        let tool = GrepTool::new();
        let tool_context = ToolContext::test();
        let dir = temp_dir_path("babata-grep-empty");
        let file = dir.join("main.rs");

        tokio::fs::create_dir_all(&dir).await.expect("create dir");
        tokio::fs::write(&file, "fn main() {}\n")
            .await
            .expect("seed file");

        let args = json!({
            "pattern": "missing_pattern",
            "path": dir.to_string_lossy(),
        })
        .to_string();

        let result = tool.execute(&args, &tool_context).await.expect("grep");
        assert_eq!(result, "No matches found.");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}
