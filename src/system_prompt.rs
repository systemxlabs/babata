use std::path::{Path, PathBuf};

use crate::{BabataResult, error::BabataError, utils::babata_dir};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemPrompt {
    pub path: PathBuf,
    pub content: String,
}

pub fn load_system_prompts() -> BabataResult<Vec<SystemPrompt>> {
    let dir = babata_dir()?.join("system_prompts");
    load_system_prompts_from_dir(&dir)
}

fn load_system_prompts_from_dir(dir: &Path) -> BabataResult<Vec<SystemPrompt>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    if !dir.is_dir() {
        return Err(BabataError::config(format!(
            "System prompt path '{}' is not a directory",
            dir.display()
        )));
    }

    let mut prompts = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|err| {
        BabataError::config(format!(
            "Failed to read system prompts directory '{}': {}",
            dir.display(),
            err
        ))
    })?;

    for entry in entries {
        let entry = entry.map_err(|err| {
            BabataError::config(format!(
                "Failed to read system prompt directory entry in '{}': {}",
                dir.display(),
                err
            ))
        })?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
            continue;
        };
        if !ext.eq_ignore_ascii_case("md") {
            continue;
        }
        let content = std::fs::read_to_string(&path).map_err(|err| {
            BabataError::config(format!(
                "Failed to read system prompt '{}': {}",
                path.display(),
                err
            ))
        })?;
        prompts.push(SystemPrompt { path, content });
    }

    prompts.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(prompts)
}
