use axum::Json;
use serde::Serialize;

use crate::{
    BabataResult,
    agent::{AgentFrontmatter, load_agents},
};

pub(super) async fn handle() -> BabataResult<Json<ListAgentsResponse>> {
    let agents = load_agents()?;
    Ok(Json(ListAgentsResponse::from_agents(agents)))
}

#[derive(Debug, Serialize)]
pub(crate) struct ListAgentsResponse {
    pub agents: Vec<AgentFrontmatter>,
}

impl ListAgentsResponse {
    pub(crate) fn from_agents(
        agents: std::collections::HashMap<String, std::sync::Arc<crate::agent::Agent>>,
    ) -> Self {
        Self {
            agents: agents
                .into_values()
                .map(|agent| agent.frontmatter.clone())
                .collect(),
        }
    }
}
