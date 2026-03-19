use chrono::Local;

use crate::{
    BabataResult,
    agent::{
        Agent,
        babata::{BabataAgent, Skill},
        codex::CodexAgent,
    },
    config::{AgentConfig, Config},
    error::BabataError,
    utils::{babata_dir, resolve_home_dir},
};

const BASE_SYSTEM_PROMPTS: &[&str] = &[include_str!("SYSTEM.md"), include_str!("SOUL.md")];

pub fn build_system_prompts(skills: &[Skill]) -> BabataResult<Vec<String>> {
    let mut sections = BASE_SYSTEM_PROMPTS
        .iter()
        .map(|section| (*section).to_string())
        .collect::<Vec<_>>();
    sections.push(build_runtime_prompt()?);

    if let Some(workspace_prompt) = load_workspace_prompt()? {
        sections.push(workspace_prompt);
    }

    if let Some(skills_prompt) = build_skills_prompt(skills) {
        sections.push(skills_prompt);
    }

    Ok(sections)
}

pub fn build_runtime_prompt() -> BabataResult<String> {
    let now = Local::now();
    Ok(format!(
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
    ))
}

pub fn build_agents_prompt(config: &Config) -> BabataResult<String> {
    let mut agent_sections = Vec::with_capacity(config.agents.len());
    for agent in &config.agents {
        let config_json = serde_json::to_string_pretty(agent).map_err(|err| {
            BabataError::config(format!(
                "Failed to serialize agent config '{}' for prompt: {}",
                agent.name(),
                err
            ))
        });

        agent_sections.push(format!(
            r#"### Agent `{}`
Description: {}

Config:
```json
{}
```"#,
            agent.name(),
            agent_description(agent),
            config_json?
        ));
    }

    Ok(format!(
        r#"Configured task agents:

{}"#,
        agent_sections.join("\n\n")
    ))
}

pub fn build_skills_prompt(skills: &[Skill]) -> Option<String> {
    let mut skill_summaries = Vec::with_capacity(skills.len());
    for skill in skills {
        let title = format!(
            "{}: {}\n  path: {}",
            skill.frontmatter.name.trim(),
            skill.frontmatter.description.trim(),
            skill.path.display()
        );
        skill_summaries.push(format!("- {title}"));
    }

    if skill_summaries.is_empty() {
        return None;
    }

    Some(format!("Available skills:\n{}", skill_summaries.join("\n")))
}

fn agent_description(agent: &AgentConfig) -> &'static str {
    match agent {
        AgentConfig::Babata(_) => BabataAgent::description(),
        AgentConfig::Codex(_) => CodexAgent::description(),
    }
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        build_agents_prompt, build_runtime_prompt, build_skills_prompt, build_system_prompts,
    };
    use crate::{
        agent::babata::{Skill, SkillFrontmatter},
        config::{AgentConfig, BabataAgentConfig, CodexAgentConfig, Config},
    };

    #[test]
    fn build_runtime_prompt_includes_runtime_fields() {
        let prompt = build_runtime_prompt().expect("build runtime prompt");

        assert!(prompt.contains("Runtime context:"));
        assert!(prompt.contains("User home directory(USER_HOME):"));
        assert!(prompt.contains("Babata home directory(BABATA_HOME):"));
        assert!(prompt.contains("Current local time:"));
        assert!(prompt.contains("User time zone:"));
        assert!(prompt.contains("Operating system:"));
        assert!(prompt.contains("CPU architecture:"));
    }

    #[test]
    fn build_system_prompts_includes_base_sections_and_runtime() {
        let prompts = build_system_prompts(&[]).expect("build system prompts");

        assert_eq!(prompts.len(), 3);
        assert!(prompts[0].contains("# AGENTS") || prompts[0].contains("# SYSTEM"));
        assert!(prompts[1].contains("Be genuinely helpful"));
        assert!(prompts[2].contains("Runtime context:"));
    }

    #[test]
    fn build_agents_prompt_includes_agent_descriptions_and_config() {
        let config = Config {
            providers: Vec::new(),
            agents: vec![
                AgentConfig::Babata(BabataAgentConfig {
                    provider: "openai".to_string(),
                    model: "gpt-4.1".to_string(),
                    memory: "simple".to_string(),
                }),
                AgentConfig::Codex(CodexAgentConfig {
                    command: "codex".to_string(),
                    workspace: "/tmp/workspace".to_string(),
                    model: Some("gpt-5-codex".to_string()),
                }),
            ],
            channels: Vec::new(),
            memory: Vec::new(),
        };

        let prompt = build_agents_prompt(&config).expect("build agents prompt");

        assert!(prompt.contains("Configured task agents:"));
        assert!(prompt.contains("Agent `babata`"));
        assert!(prompt.contains("general tasks, task orchestration"));
        assert!(prompt.contains("\"name\": \"codex\""));
        assert!(prompt.contains("\"workspace\": \"/tmp/workspace\""));
    }

    #[test]
    fn build_skills_prompt_includes_skill_summaries() {
        let prompt = build_skills_prompt(&[
            Skill {
                path: PathBuf::from("/tmp/skills/research/SKILL.md"),
                frontmatter: SkillFrontmatter {
                    name: "research".to_string(),
                    description: "Find primary sources".to_string(),
                },
                body: String::new(),
            },
            Skill {
                path: PathBuf::from("/tmp/skills/coding/SKILL.md"),
                frontmatter: SkillFrontmatter {
                    name: "coding".to_string(),
                    description: "Implement code changes".to_string(),
                },
                body: String::new(),
            },
        ])
        .expect("build skills prompt");

        assert!(prompt.contains("Available skills:"));
        assert!(prompt.contains("- research: Find primary sources"));
        assert!(prompt.contains("path: /tmp/skills/research/SKILL.md"));
        assert!(prompt.contains("- coding: Implement code changes"));
        assert!(prompt.contains("path: /tmp/skills/coding/SKILL.md"));
    }

    #[test]
    fn build_skills_prompt_returns_none_for_empty_skills() {
        assert!(build_skills_prompt(&[]).is_none());
    }
}
