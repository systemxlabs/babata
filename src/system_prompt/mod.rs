use std::{collections::HashMap, sync::Arc};

use crate::{
    BabataResult,
    agent::Agent,
    channel::ChannelConfig,
    skill::Skill,
    tool::ToolSpec,
    utils::{babata_dir, channel_dir, task_dir, user_home_dir},
};
use chrono::Local;
use uuid::Uuid;

pub const BABATA_SYSTEM_DESCRIPTION: &str = include_str!("SYSTEM.md");

pub fn build_system_prompts(
    channel_configs: &[ChannelConfig],
    agents: &HashMap<String, Arc<Agent>>,
    skills: &[Skill],
    agent_body: &str,
    tool_specs: &[ToolSpec],
    task_id: Uuid,
) -> BabataResult<Vec<String>> {
    let mut sections = vec![
        agent_body.to_string(),
        BABATA_SYSTEM_DESCRIPTION.to_string(),
        build_environment_prompt(task_id)?,
        build_agents_prompt(agents),
        build_channels_prompt(channel_configs)?,
    ];

    if let Some(skills_prompt) = build_skills_prompt(skills) {
        sections.push(skills_prompt);
    }

    if let Some(tools_prompt) = build_tools_prompt(tool_specs) {
        sections.push(tools_prompt);
    }

    Ok(sections)
}

pub fn build_environment_prompt(task_id: Uuid) -> BabataResult<String> {
    let now = Local::now();
    Ok(format!(
        r#"# Environment
- User home directory(USER_HOME): {}
- Babata home directory(BABATA_HOME): {}
- Task home directory(TASK_HOME): {}
- Current working directory(CWD): {}
- User time zone: {}
- Operating system: {}
- CPU architecture: {}"#,
        user_home_dir()?.display(),
        babata_dir()?.display(),
        std::env::current_dir()?.display(),
        task_dir(task_id)?.display(),
        now.format("%Z (%:z)"),
        std::env::consts::OS,
        std::env::consts::ARCH
    ))
}

pub fn build_agents_prompt(agents: &HashMap<String, Arc<Agent>>) -> String {
    let mut agent_sections = Vec::with_capacity(agents.len());
    for agent in agents.values() {
        agent_sections.push(format!(
            "- `{}`: {}",
            agent.frontmatter.name, agent.frontmatter.description,
        ));
    }

    format!(
        r#"# Configured agents
{}

You can chose an agent from the list above to use for tasks."#,
        agent_sections.join("\n")
    )
}

pub fn build_channels_prompt(channel_configs: &[ChannelConfig]) -> BabataResult<String> {
    let mut channel_sections = Vec::with_capacity(channel_configs.len());
    for channel in channel_configs {
        let description = match channel {
            ChannelConfig::Telegram(telegram) => format!(
                "{}: receives messages from Telegram user (id: {}) via bot (token: {})",
                channel.name(),
                telegram.user_id,
                telegram.bot_token
            ),
            ChannelConfig::Wechat(wechat) => {
                let channel_dir = channel_dir(channel.name())?;
                format!(
                    "{}: receives messages from Wechat user (id: {}) via Wechat iLink bot (token: {}). Read file `{}/latest_context_token` to get latest context token",
                    channel.name(),
                    wechat.user_id,
                    wechat.bot_token,
                    channel_dir.display()
                )
            }
        };
        channel_sections.push(format!("- {description}"));
    }

    Ok(format!(
        r#"# Configured channels
{}"#,
        channel_sections.join("\n")
    ))
}

pub fn build_skills_prompt(skills: &[Skill]) -> Option<String> {
    let mut skill_summaries = Vec::with_capacity(skills.len());
    for skill in skills {
        let title = format!(
            "{} (from {}): {}",
            skill.frontmatter.name,
            skill.path.display(),
            skill.frontmatter.description,
        );
        skill_summaries.push(format!("- {title}"));
    }

    if skill_summaries.is_empty() {
        return None;
    }

    Some(format!(
        r#"# Available skills
{}"#,
        skill_summaries.join("\n")
    ))
}

pub fn build_tools_prompt(tool_specs: &[ToolSpec]) -> Option<String> {
    if tool_specs.is_empty() {
        return None;
    }

    let mut tool_summaries = Vec::with_capacity(tool_specs.len());
    for tool in tool_specs {
        tool_summaries.push(format!("- `{}`: {}", tool.name, tool.description));
    }

    Some(format!(
        r#"# Available tools
{}"#,
        tool_summaries.join("\n")
    ))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use schemars::JsonSchema;
    use serde::Deserialize;
    use uuid::Uuid;

    use super::{
        build_channels_prompt, build_environment_prompt, build_skills_prompt, build_tools_prompt,
    };
    use crate::{
        channel::{ChannelConfig, TelegramChannelConfig, WechatChannelConfig},
        skill::{Skill, SkillFrontmatter},
        tool::ToolSpec,
    };

    #[test]
    fn build_environment_prompt_includes_environment_fields() {
        let prompt = build_environment_prompt(Uuid::new_v4()).expect("build environment prompt");

        assert!(prompt.contains("# Environment"));
        assert!(prompt.contains("User home directory(USER_HOME):"));
        assert!(prompt.contains("Babata home directory(BABATA_HOME):"));
        assert!(prompt.contains("Current working directory(CWD):"));
        assert!(prompt.contains("User time zone:"));
        assert!(prompt.contains("Operating system:"));
        assert!(prompt.contains("CPU architecture:"));
    }

    #[test]
    fn build_channels_prompt_includes_channel_capabilities() {
        let channel_configs = vec![
            ChannelConfig::Telegram(TelegramChannelConfig {
                bot_token: "token".to_string(),
                user_id: 123456,
            }),
            ChannelConfig::Wechat(WechatChannelConfig {
                bot_token: "token".to_string(),
                user_id: "wxid_123".to_string(),
            }),
        ];

        let prompt = build_channels_prompt(&channel_configs).unwrap();

        assert!(prompt.contains("# Configured channels"));
        assert!(prompt.contains("Telegram: receives messages from Telegram user (id: 123456)"));
        assert!(prompt.contains("via bot (token: token)"));
        assert!(prompt.contains("Wechat: receives messages from Wechat user (id: wxid_123)"));
        assert!(prompt.contains("Read file `"));
        assert!(prompt.contains("latest_context_token"));
    }

    #[test]
    fn build_channels_prompt_returns_header_when_empty() {
        let prompt = build_channels_prompt(&[]).unwrap();

        assert_eq!(prompt, "# Configured channels\n");
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

        assert!(prompt.contains("# Available skills"));
        assert!(
            prompt
                .contains("- research (from /tmp/skills/research/SKILL.md): Find primary sources")
        );
        assert!(
            prompt.contains("- coding (from /tmp/skills/coding/SKILL.md): Implement code changes")
        );
    }

    #[test]
    fn build_skills_prompt_returns_none_for_empty_skills() {
        assert!(build_skills_prompt(&[]).is_none());
    }

    #[test]
    fn build_tools_prompt_includes_tool_summaries() {
        #[allow(dead_code)]
        #[derive(Deserialize, JsonSchema)]
        struct ReadFileArgs {
            path: String,
        }

        #[allow(dead_code)]
        #[derive(Deserialize, JsonSchema)]
        struct CreateTaskArgs {
            prompt: String,
        }

        let prompt = build_tools_prompt(&[
            ToolSpec {
                name: "read_file".to_string(),
                description: "Read a file".to_string(),
                parameters: schemars::schema_for!(ReadFileArgs),
            },
            ToolSpec {
                name: "create_task".to_string(),
                description: "Create a task".to_string(),
                parameters: schemars::schema_for!(CreateTaskArgs),
            },
        ])
        .expect("build tools prompt");

        assert!(prompt.contains("# Available tools"));
        assert!(prompt.contains("- `read_file`: Read a file"));
        assert!(prompt.contains("- `create_task`: Create a task"));
    }

    #[test]
    fn build_tools_prompt_returns_none_for_empty_tools() {
        assert!(build_tools_prompt(&[]).is_none());
    }
}
