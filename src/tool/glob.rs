use schemars::JsonSchema;
use serde::Deserialize;
use std::cmp::Reverse;
use std::path::PathBuf;

use crate::{
    BabataResult,
    error::BabataError,
    tool::{Tool, ToolContext, ToolSpec, parse_tool_args},
};

const MAX_RESULTS: usize = 100;

#[derive(Debug, Clone)]
pub struct GlobTool {
    spec: ToolSpec,
}

impl Default for GlobTool {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobTool {
    pub fn new() -> Self {
        Self {
            spec: ToolSpec {
                name: "glob".to_string(),
                description: format!(
                    "Find files matching a glob pattern. Supports ** for recursive matching (e.g. '**/*.py'). Returns at most {} matches.",
                    MAX_RESULTS
                ),
                parameters: schemars::schema_for!(GlobArgs),
            },
        }
    }
}

#[async_trait::async_trait]
impl Tool for GlobTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: &str, _context: &ToolContext<'_>) -> BabataResult<String> {
        let args: GlobArgs = parse_tool_args(args)?;

        let path = args
            .path
            .map(|p| shellexpand::tilde(&p).to_string())
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| ".".to_string())
            });

        let base = PathBuf::from(&path);
        if !base.is_dir() {
            return Err(BabataError::tool(format!("'{}' is not a directory", path)));
        }

        // Collect matching files
        let pattern_with_base = base.join(&args.pattern);
        let hits = glob::glob(pattern_with_base.to_str().unwrap_or(&args.pattern))
            .map_err(|e| BabataError::tool(format!("Invalid glob pattern: {}", e)))?
            .filter_map(|entry| entry.ok())
            .collect::<Vec<_>>();

        // Sort by mtime, newest first
        let mut hits_with_mtime: Vec<(PathBuf, u64)> = hits
            .into_iter()
            .filter_map(|p| {
                p.metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| (p, d.as_secs()))
            })
            .collect();
        hits_with_mtime.sort_by_key(|entry| Reverse(entry.1));

        let total = hits_with_mtime.len();
        let shown: Vec<_> = hits_with_mtime.into_iter().take(MAX_RESULTS).collect();

        let lines: Vec<String> = shown
            .into_iter()
            .map(|(p, _)| p.to_string_lossy().to_string())
            .collect();

        let mut result = lines.join("\n");

        if total > MAX_RESULTS {
            result.push_str(&format!(
                "\n... ({} matches total, showing first {})",
                total, MAX_RESULTS
            ));
        }

        if result.is_empty() {
            result = "No files matched.".to_string();
        }

        Ok(result)
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct GlobArgs {
    #[schemars(description = "Glob pattern, e.g. '**/*.py' or 'src/**/*.ts'")]
    pattern: String,
    #[schemars(description = "Directory to search in (default: current working directory)")]
    path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{GlobArgs, parse_tool_args};
    use serde_json::json;

    fn parse_glob_args(args: &str) -> crate::BabataResult<(String, String)> {
        let args: GlobArgs = parse_tool_args(args)?;
        let path = args
            .path
            .map(|p| shellexpand::tilde(&p).to_string())
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| ".".to_string())
            });
        Ok((args.pattern, path))
    }

    #[test]
    fn validate_args_extracts_pattern_and_path() {
        let (pattern, path) =
            parse_glob_args(&json!({ "pattern": "**/*.rs", "path": "/tmp" }).to_string())
                .expect("parse args");
        assert_eq!(pattern, "**/*.rs");
        assert_eq!(path, "/tmp");
    }

    #[test]
    fn validate_args_expands_tilde_in_path() {
        let (pattern, path) =
            parse_glob_args(&json!({ "pattern": "*.txt", "path": "~" }).to_string())
                .expect("parse args");
        assert_eq!(pattern, "*.txt");
        assert!(!path.starts_with('~'));
    }

    #[test]
    fn validate_args_uses_cwd_when_path_missing() {
        let (pattern, path) =
            parse_glob_args(&json!({ "pattern": "*.md" }).to_string()).expect("parse args");
        assert_eq!(pattern, "*.md");
        assert!(!path.is_empty());
    }

    #[test]
    fn validate_args_rejects_missing_pattern() {
        let err =
            parse_glob_args(&json!({ "path": "/tmp" }).to_string()).expect_err("missing pattern");
        assert!(
            err.to_string().contains("Invalid tool arguments")
                && err.to_string().contains("missing field `pattern`")
        );
    }
}
