use crate::{
    BabataResult,
    config::{AgentConfig, Config},
    error::BabataError,
};

use super::Args;

pub fn add(_args: &Args, agent_config_json: &str) {
    if let Err(err) = run_add(agent_config_json) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

pub fn delete(_args: &Args, name: &str) {
    if let Err(err) = run_delete(name) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

pub fn list(_args: &Args) {
    if let Err(err) = run_list() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run_add(agent_config_json: &str) -> BabataResult<()> {
    let agent_config: AgentConfig = serde_json::from_str(agent_config_json).map_err(|err| {
        BabataError::config(format!(
            "Invalid agent config JSON '{}': {}",
            agent_config_json, err
        ))
    })?;

    let mut config = Config::load()?;
    let agent_name = agent_config.name.clone();
    config.upsert_agent(agent_config);
    config.validate()?;
    config.save()?;

    println!("Added/updated agent '{}' in config", agent_name);
    Ok(())
}

fn run_delete(name: &str) -> BabataResult<()> {
    let mut config = Config::load()?;

    let index = config
        .agents
        .iter()
        .position(|agent| agent.name == name)
        .ok_or_else(|| BabataError::config(format!("Agent '{}' not found in config", name)))?;

    let deleted = config.agents.remove(index);
    config.validate()?;
    config.save()?;

    println!("Deleted agent '{}'", deleted.name);
    Ok(())
}

fn run_list() -> BabataResult<()> {
    let config = Config::load()?;

    for agent_config in &config.agents {
        let payload = serde_json::to_string(agent_config).map_err(|err| {
            BabataError::internal(format!(
                "Failed to serialize agent '{}' config to JSON: {}",
                agent_config.name, err
            ))
        })?;
        println!("{payload}");
    }

    Ok(())
}
