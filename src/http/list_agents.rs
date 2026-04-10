use axum::Json;
use serde::Serialize;

use crate::{BabataResult, agent::{load_agents, AgentFrontmatter}};

pub(super) async fn handle() -> BabataResult<Json<ListAgentsResponse>> {
    let agents = load_agents()?;
    Ok(Json(ListAgentsResponse::from_agents(agents)))
}

#[derive(Debug, Serialize)]
pub(crate) struct AgentResponse {
    pub name: String,
    pub description: String,
    pub provider: String,
    pub model: String,
    pub allowed_tools: Vec<String>,
    pub default: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct ListAgentsResponse {
    pub agents: Vec<AgentResponse>,
}

impl ListAgentsResponse {
    pub(crate) fn from_agents(agents: std::collections::HashMap<String, std::sync::Arc<crate::agent::Agent>>) -> Self {
        Self {
            agents: agents
                .into_values()
                .map(|agent| AgentResponse::from_frontmatter(&agent.frontmatter))
                .collect(),
        }
    }
}

impl AgentResponse {
    pub(crate) fn from_frontmatter(frontmatter: &AgentFrontmatter) -> Self {
        Self {
            name: frontmatter.name.clone(),
            description: frontmatter.description.clone(),
            provider: frontmatter.provider.clone(),
            model: frontmatter.model.clone(),
            allowed_tools: frontmatter.allowed_tools.clone(),
            default: frontmatter.default.unwrap_or(false),
        }
    }
}
