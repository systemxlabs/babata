use chrono::Local;

use crate::{
    BabataResult,
    agent::babata::Skill,
    error::BabataError,
    utils::{babata_dir, resolve_home_dir},
};

const BASE_SYSTEM_PROMPT: &str = include_str!("AGENTS.md");
const SOUL_PROMPT: &str = include_str!("SOUL.md");

pub fn build_system_prompt(skills: &[Skill]) -> BabataResult<String> {
    let mut sections = Vec::new();
    sections.push(BASE_SYSTEM_PROMPT.to_string());
    sections.push(SOUL_PROMPT.to_string());

    let now = Local::now();
    let runtime_context = format!(
        r#"Runtime context:
- User home directory(USER_HOME): {}
- Babata home directory(BABATA_HOME): {}
- Current local time: {}
- User time zone: {}
- Operating system: {}
- CPU architecture: {}"#,
        resolve_home_dir()?.display(),
        babata_dir()?.display(),
        now.to_rfc3339(),
        now.format("%Z (%:z)"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    sections.push(runtime_context);

    if let Some(workspace_prompt) = load_workspace_prompt()? {
        sections.push(workspace_prompt);
    }

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

    Ok(sections.join("\n\n"))
}

fn load_workspace_prompt() -> BabataResult<Option<String>> {
    let babata_home = babata_dir()?;
    let workspace_prompt_path = babata_home.join("workspace").join("workspace.md");
    if !workspace_prompt_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&workspace_prompt_path).map_err(|err| {
        BabataError::config(format!(
            "Failed to read system prompt '{}': {}",
            workspace_prompt_path.display(),
            err
        ))
    })?;
    let content = content.trim();
    if content.is_empty() {
        return Ok(None);
    }

    Ok(Some(content.to_string()))
}
