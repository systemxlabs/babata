use axum::extract::Path;

use crate::{
    BabataResult,
    agent::{agent_exists, delete_agent, load_agents},
};

pub(super) async fn handle(Path(name): Path<String>) -> BabataResult<()> {
    // Check if agent exists
    if !agent_exists(&name) {
        return Err(crate::error::BabataError::not_found(format!(
            "Agent '{}' not found",
            name
        )));
    }

    // Load all agents to check if this is the only agent
    let agents = load_agents()?;

    // Check if trying to delete the only default agent
    if let Some(agent) = agents.get(&name) {
        let is_default = agent.frontmatter.default == Some(true);
        let is_only_agent = agents.len() == 1;

        if is_default && is_only_agent {
            return Err(crate::error::BabataError::invalid_input(
                "Cannot delete the only default agent. System must have at least one agent.",
            ));
        }
    }

    delete_agent(&name)?;
    Ok(())
}
