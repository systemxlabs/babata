use crate::{
    BabataResult,
    agent::{
        prompt::{
            BABATA_SYSTEM_DESCRIPTION, build_agents_prompt, build_channels_prompt,
            build_runtime_prompt, build_skills_prompt, load_workspace_prompt,
        },
        skill::Skill,
    },
    config::Config,
};

const SOUL_PROMPT: &str = include_str!("SOUL.md");

pub fn build_system_prompts(config: &Config, skills: &[Skill]) -> BabataResult<Vec<String>> {
    let mut sections = vec![
        SOUL_PROMPT.to_string(),
        BABATA_SYSTEM_DESCRIPTION.to_string(),
        build_runtime_prompt()?,
        build_agents_prompt(config),
        build_channels_prompt(config)?,
    ];

    if let Some(workspace_prompt) = load_workspace_prompt()? {
        sections.push(workspace_prompt);
    }

    if let Some(skills_prompt) = build_skills_prompt(skills) {
        sections.push(skills_prompt);
    }

    Ok(sections)
}

#[cfg(test)]
mod tests {
    use super::build_system_prompts;
    use crate::config::{AgentConfig, BabataAgentConfig, Config};

    #[test]
    fn build_system_prompts_includes_base_sections_runtime_and_agents() {
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

        let prompts = build_system_prompts(&config, &[]).expect("build system prompts");

        // BASE_SYSTEM_PROMPTS (2) + runtime + agents + channels + workspace (if exists)
        assert!(prompts.len() >= 5);
        assert!(prompts[0].contains("# SOUL"));
        assert!(prompts[1].contains("# Babata System"));
        assert!(prompts[2].contains("Runtime context:"));
        assert!(prompts[3].contains("Configured agents:"));
        assert!(prompts[4].contains("Configured channels:"));
    }
}
