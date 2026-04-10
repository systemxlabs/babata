use axum::{
    Json,
    extract::Path,
};
use serde::Serialize;

use crate::BabataResult;
use crate::agent::load_agent_by_name;
use crate::error::BabataError;

#[derive(Debug, Serialize)]
pub(crate) struct GetAgentResponse {
    pub name: String,
    pub description: String,
    pub provider: String,
    pub model: String,
    pub allowed_tools: Vec<String>,
    pub default: bool,
    pub body: String,
}

pub(super) async fn handle(
    Path(name): Path<String>,
) -> BabataResult<Json<GetAgentResponse>> {
    let agent = load_agent_by_name(&name).map_err(|err| {
        // Convert config error (agent not found) to not found error for proper 404 response
        if err.to_string().contains("not found") {
            BabataError::not_found(format!("Agent '{}' not found", name))
        } else {
            err
        }
    })?;

    Ok(Json(GetAgentResponse {
        name: agent.frontmatter.name.clone(),
        description: agent.frontmatter.description.clone(),
        provider: agent.frontmatter.provider.clone(),
        model: agent.frontmatter.model.clone(),
        allowed_tools: agent.frontmatter.allowed_tools.clone(),
        default: agent.frontmatter.default.unwrap_or(false),
        body: agent.body.clone(),
    }))
}
