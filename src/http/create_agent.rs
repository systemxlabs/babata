use axum::{Json, extract::State};
use serde::Deserialize;

use crate::{
    BabataResult,
    agent::{AgentFrontmatter, agent_exists, load_agents, save_agent},
    error::BabataError,
};

use super::HttpApp;

pub(super) async fn handle(
    State(_state): State<HttpApp>,
    Json(request): Json<CreateAgentRequest>,
) -> BabataResult<()> {
    // Validate name is not empty
    if request.name.trim().is_empty() {
        return Err(BabataError::invalid_input("name cannot be empty"));
    }

    // Check if agent already exists
    if agent_exists(&request.name) {
        return Err(BabataError::invalid_input(format!(
            "Agent '{}' already exists",
            request.name
        )));
    }

    // If setting as default, check if another default agent already exists
    if request.default {
        let agents = load_agents()?;
        if agents
            .values()
            .any(|agent| agent.frontmatter.default == Some(true))
        {
            return Err(BabataError::invalid_input(
                "Another default agent already exists. Only one agent can be default.",
            ));
        }
    }

    // Create the agent frontmatter
    let frontmatter = AgentFrontmatter {
        name: request.name.clone(),
        description: request.description,
        provider: request.provider,
        model: request.model,
        allowed_tools: request.allowed_tools,
        default: if request.default { Some(true) } else { None },
    };

    // Save the agent
    save_agent(&frontmatter, &request.body)?;

    Ok(())
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateAgentRequest {
    pub name: String,
    pub description: String,
    pub provider: String,
    pub model: String,
    pub allowed_tools: Vec<String>,
    #[serde(default)]
    pub default: bool,
    pub body: String,
}
