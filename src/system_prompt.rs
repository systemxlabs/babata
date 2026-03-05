use std::path::{Path, PathBuf};

use chrono::Local;

use crate::{BabataResult, config::Config, error::BabataError, skill::Skill, utils::babata_dir};

pub fn build_system_prompt(
    config: &Config,
    system_prompt_files: &[SystemPromptFile],
    skills: &[Skill],
) -> String {
    let mut sections = Vec::new();

    for prompt_file in system_prompt_files {
        let content = prompt_file.content.trim();
        if !content.is_empty() {
            sections.push(content.to_string());
        }
    }

    let now = Local::now();
    let runtime_context = format!(
        r#"Runtime context:
- User home directory(USER_HOME): {}
- Babata home directory(BABATA_HOME): {}
- Current local time: {}
- User time zone: {}
- Operating system: {}
- CPU architecture: {}"#,
        config.user_home,
        format!("{}/.babata/", config.user_home),
        now.to_rfc3339(),
        now.format("%Z (%:z)"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    sections.push(runtime_context);

    let mut skill_summaries = Vec::new();
    for skill in skills {
        let title = format!(
            "{}: {}",
            skill.frontmatter.name.trim(),
            skill.frontmatter.description.trim()
        );
        skill_summaries.push(format!("- {title}"));
    }

    if !skill_summaries.is_empty() {
        sections.push(format!("Available skills:\n{}", skill_summaries.join("\n")));
    }

    sections.join("\n\n")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemPromptFile {
    pub path: PathBuf,
    pub content: String,
}

pub fn load_system_prompt_files() -> BabataResult<Vec<SystemPromptFile>> {
    let dir = babata_dir()?.join("system_prompts");
    load_system_prompt_files_from_dir(&dir)
}

fn load_system_prompt_files_from_dir(dir: &Path) -> BabataResult<Vec<SystemPromptFile>> {
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
        prompts.push(SystemPromptFile { path, content });
    }

    prompts.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(prompts)
}
