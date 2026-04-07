use serde_json::{Value, json};
use std::path::PathBuf;

use crate::{
    BabataResult,
    error::BabataError,
    tool::{Tool, ToolContext, ToolSpec},
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
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Glob pattern, e.g. '**/*.py' or 'src/**/*.ts'"
                        },
                        "path": {
                            "type": "string",
                            "description": "Directory to search in (default: current working directory)"
                        }
                    },
                    "required": ["pattern"]
                }),
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
        let (pattern, path) = parse_args(args)?;

        let base = PathBuf::from(&path);
        if !base.is_dir() {
            return Err(BabataError::tool(format!("'{}' is not a directory", path)));
        }

        // Collect matching files
        let pattern_with_base = base.join(&pattern);
        let hits = glob::glob(pattern_with_base.to_str().unwrap_or(&pattern))
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
        hits_with_mtime.sort_by(|a, b| b.1.cmp(&a.1));

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

fn parse_args(args: &str) -> BabataResult<(String, String)> {
    let args: Value = serde_json::from_str(args)?;
    let pattern = args["pattern"]
        .as_str()
        .ok_or_else(|| BabataError::tool("Missing required parameter: pattern"))?;

    let path = args["path"]
        .as_str()
        .map(|p| shellexpand::tilde(p).to_string())
        .unwrap_or_else(|| {
            std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string())
        });

    Ok((pattern.to_string(), path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_args_extracts_pattern_and_path() {
        let (pattern, path) =
            parse_args(&json!({ "pattern": "**/*.rs", "path": "/tmp" }).to_string())
                .expect("parse args");
        assert_eq!(pattern, "**/*.rs");
        assert_eq!(path, "/tmp");
    }

    #[test]
    fn parse_args_expands_tilde_in_path() {
        let (pattern, path) = parse_args(&json!({ "pattern": "*.txt", "path": "~" }).to_string())
            .expect("parse args");
        assert_eq!(pattern, "*.txt");
        assert!(!path.starts_with('~'));
    }

    #[test]
    fn parse_args_uses_cwd_when_path_missing() {
        let (pattern, path) =
            parse_args(&json!({ "pattern": "*.md" }).to_string()).expect("parse args");
        assert_eq!(pattern, "*.md");
        assert!(!path.is_empty());
    }

    #[test]
    fn parse_args_rejects_missing_pattern() {
        let err = parse_args(&json!({ "path": "/tmp" }).to_string()).expect_err("missing pattern");
        assert!(
            err.to_string()
                .contains("Missing required parameter: pattern")
        );
    }
}
