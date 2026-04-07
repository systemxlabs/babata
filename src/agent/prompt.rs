use chrono::Local;

use crate::{
    BabataResult,
    agent::{Agent, babata::BabataAgent, skill::Skill},
    channel::load_wechat_latest_context_token,
    config::{AgentConfig, ChannelConfig, Config},
    error::BabataError,
    utils::{babata_dir, resolve_home_dir},
};

pub const BABATA_SYSTEM_DESCRIPTION: &str = include_str!("SYSTEM.md");

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

pub fn build_agents_prompt(config: &Config) -> String {
    let mut agent_sections = Vec::with_capacity(config.agents.len());
    for agent in &config.agents {
        let description = match agent {
            AgentConfig::Babata(_) => BabataAgent::description(),
        };
        agent_sections.push(format!("- `{}`: {}", agent.name(), description,));
    }

    format!(
        r#"Configured agents:
{}

You can chose an agent from the list above to use for tasks."#,
        agent_sections.join("\n")
    )
}

pub fn build_channels_prompt(config: &Config) -> BabataResult<String> {
    let mut channel_sections = Vec::with_capacity(config.channels.len());
    for channel in &config.channels {
        let description = match channel {
            ChannelConfig::Telegram(telegram) => format!(
                "`telegram`: receives messages from Telegram user_id `{}`",
                telegram.user_id
            ),
            ChannelConfig::Wechat(wechat) => {
                let latest_context_token = load_wechat_latest_context_token()?
                    .unwrap_or_else(|| "unavailable".to_string());
                format!(
                    "`wechat`: receives messages from Wechat user_id `{}`; bot token: `{}`; latest context token: `{}`",
                    wechat.user_id, wechat.bot_token, latest_context_token
                )
            }
        };
        channel_sections.push(format!("- {description}"));
    }

    if channel_sections.is_empty() {
        return Ok("Configured channels:\n- none".to_string());
    }

    Ok(format!(
        r#"Configured channels:
{}
"#,
        channel_sections.join("\n")
    ))
}

pub fn build_skills_prompt(skills: &[Skill]) -> Option<String> {
    let mut skill_summaries = Vec::with_capacity(skills.len());
    for skill in skills {
        let title = format!(
            "{}: {}\n  path: {}",
            skill.frontmatter.name,
            skill.frontmatter.description,
            skill.path.display()
        );
        skill_summaries.push(format!("- {title}"));
    }

    if skill_summaries.is_empty() {
        return None;
    }

    Some(format!("Available skills:\n{}", skill_summaries.join("\n")))
}

pub fn load_workspace_prompt() -> BabataResult<Option<String>> {
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
    if content.is_empty() {
        return Ok(None);
    }

    Ok(Some(content.to_string()))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        build_agents_prompt, build_channels_prompt, build_runtime_prompt, build_skills_prompt,
    };
    use crate::{
        agent::skill::{Skill, SkillFrontmatter},
        config::{
            AgentConfig, BabataAgentConfig, ChannelConfig, Config, TelegramChannelConfig,
            WechatChannelConfig,
        },
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
    fn build_agents_prompt_includes_agent_descriptions_and_config() {
        let config = Config {
            providers: Vec::new(),
            agents: vec![AgentConfig::Babata(BabataAgentConfig {
                provider: "openai".to_string(),
                model: "gpt-4.1".to_string(),
                memory: "simple".to_string(),
            })],
            channels: Vec::new(),
            memory: Vec::new(),
        };

        let prompt = build_agents_prompt(&config);

        assert!(prompt.contains("Configured agents:"));
        assert!(prompt.contains("`babata`"));
        assert!(prompt.contains("general tasks, task orchestration"));
    }

    #[test]
    fn build_channels_prompt_includes_channel_capabilities() {
        let config = Config {
            providers: Vec::new(),
            agents: Vec::new(),
            channels: vec![
                ChannelConfig::Telegram(TelegramChannelConfig {
                    bot_token: "token".to_string(),
                    user_id: 123456,
                }),
                ChannelConfig::Wechat(WechatChannelConfig {
                    bot_token: "token".to_string(),
                    user_id: "wxid_123".to_string(),
                }),
            ],
            memory: Vec::new(),
        };

        let prompt = build_channels_prompt(&config).unwrap();

        assert!(prompt.contains("Configured channels:"));
        assert!(prompt.contains("`telegram`"));
        assert!(prompt.contains("Telegram user_id `123456`"));
        assert!(prompt.contains("`wechat`"));
        assert!(prompt.contains("Wechat user_id `wxid_123`"));
        assert!(prompt.contains("latest context token: `"));
    }

    #[test]
    fn build_channels_prompt_returns_none_config_when_empty() {
        let config = Config {
            providers: Vec::new(),
            agents: Vec::new(),
            channels: Vec::new(),
            memory: Vec::new(),
        };

        let prompt = build_channels_prompt(&config).unwrap();

        assert_eq!(prompt, "Configured channels:\n- none");
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
