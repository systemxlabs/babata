use regex::Regex;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

use crate::{
    BabataResult,
    agent::babata::{Tool, ToolContext, ToolSpec},
    error::BabataError,
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
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Regex pattern to search for"
                        },
                        "path": {
                            "type": "string",
                            "description": "File or directory to search (default: current working directory)"
                        },
                        "include": {
                            "type": "string",
                            "description": "Only search files matching this glob (e.g. '*.py')"
                        }
                    },
                    "required": ["pattern"]
                }),
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
        let (pattern, path, include) = parse_args(args)?;

        let regex =
            Regex::new(&pattern).map_err(|e| BabataError::tool(format!("Invalid regex: {}", e)))?;

        let base = PathBuf::from(&path);
        if !base.exists() {
            return Err(BabataError::tool(format!("'{}' not found", path)));
        }

        let files: Vec<PathBuf> = if base.is_file() {
            vec![base]
        } else {
            walk(&base, include.as_deref())
        };

        let mut matches = Vec::new();

        for fp in files {
            let text = match fs::read_to_string(&fp) {
                Ok(content) => content,
                Err(_) => continue, // skip binary or unreadable files
            };

            for (lineno, line) in text.lines().enumerate() {
                let line_num = lineno + 1;
                if regex.is_match(line) {
                    matches.push(format!(
                        "{}:{}: {}",
                        fp.display(),
                        line_num,
                        line.trim_end()
                    ));
                    if matches.len() >= MAX_MATCHES {
                        matches.push("... (match limit reached)".to_string());
                        return Ok(matches.join("\n"));
                    }
                }
            }
        }

        if matches.is_empty() {
            Ok("No matches found.".to_string())
        } else {
            Ok(matches.join("\n"))
        }
    }
}

fn parse_args(args: &str) -> BabataResult<(String, String, Option<String>)> {
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

    let include = args["include"].as_str().map(|s| s.to_string());

    Ok((pattern.to_string(), path, include))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_args_extracts_pattern_path_and_include() {
        let (pattern, path, include) =
            parse_args(&json!({ "pattern": "foo", "path": "/tmp", "include": "*.rs" }).to_string())
                .expect("parse args");
        assert_eq!(pattern, "foo");
        assert_eq!(path, "/tmp");
        assert_eq!(include, Some("*.rs".to_string()));
    }

    #[test]
    fn parse_args_expands_tilde_in_path() {
        let (pattern, path, include) =
            parse_args(&json!({ "pattern": "test", "path": "~" }).to_string()).expect("parse args");
        assert_eq!(pattern, "test");
        assert!(!path.starts_with('~'));
        assert_eq!(include, None);
    }

    #[test]
    fn parse_args_uses_cwd_when_path_missing() {
        let (pattern, path, include) =
            parse_args(&json!({ "pattern": "hello" }).to_string()).expect("parse args");
        assert_eq!(pattern, "hello");
        assert!(!path.is_empty());
        assert_eq!(include, None);
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
