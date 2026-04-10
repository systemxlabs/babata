use axum::{Json, extract::Path, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};

use crate::{
    BabataResult,
    agent::{AgentFrontmatter, agent_exists, load_agent_by_name, save_agent},
    error::BabataError,
};

#[derive(Debug, Deserialize)]
pub(crate) struct UpdateAgentRequest {
    pub description: String,
    pub provider: String,
    pub model: String,
    pub allowed_tools: Vec<String>,
    pub default: bool,
    pub body: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct UpdateAgentResponse {
    pub name: String,
}

pub(super) async fn handle(
    Path(name): Path<String>,
    Json(request): Json<UpdateAgentRequest>,
) -> BabataResult<impl IntoResponse> {
    // Check if agent exists
    if !agent_exists(&name) {
        return Err(BabataError::not_found(format!("Agent '{}' not found", name)));
    }

    // Load existing agent to check if default status is changing
    let existing_agent = load_agent_by_name(&name)?;
    let old_default = existing_agent.frontmatter.default == Some(true);
    let new_default = request.default;

    // If setting this agent as default, we need to handle the old default agent
    if new_default && !old_default {
        // Unset default on the current default agent
        if let Err(err) = unset_current_default_agent(&name).await {
            log::warn!("Failed to unset current default agent: {}", err);
        }
    }

    // Build the updated frontmatter
    let frontmatter = AgentFrontmatter {
        name: name.clone(),
        description: request.description,
        provider: request.provider,
        model: request.model,
        allowed_tools: request.allowed_tools,
        default: Some(new_default),
    };

    // Save the updated agent
    save_agent(&name, &frontmatter, &request.body)?;

    Ok((StatusCode::OK, Json(UpdateAgentResponse { name })))
}

/// Unset the current default agent (set default to false)
/// This is called when a new agent is being set as default
async fn unset_current_default_agent(excluding: &str) -> BabataResult<()> {
    use crate::agent::load_agents;

    let agents = load_agents()?;

    for (agent_name, agent) in agents.iter() {
        if agent_name == excluding {
            continue;
        }

        if agent.frontmatter.default == Some(true) {
            let new_frontmatter = AgentFrontmatter {
                name: agent.frontmatter.name.clone(),
                description: agent.frontmatter.description.clone(),
                provider: agent.frontmatter.provider.clone(),
                model: agent.frontmatter.model.clone(),
                allowed_tools: agent.frontmatter.allowed_tools.clone(),
                default: Some(false),
            };
            save_agent(agent_name, &new_frontmatter, &agent.body)?;
            log::info!("Unset default agent '{}'", agent_name);
            break;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_update_agent_request_deserialization() {
        let json = json!({
            "description": "Updated agent description",
            "provider": "openai",
            "model": "gpt-4",
            "allowed_tools": ["shell", "read_file"],
            "default": true,
            "body": "Updated agent body"
        });

        let request: UpdateAgentRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.description, "Updated agent description");
        assert_eq!(request.provider, "openai");
        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.allowed_tools, vec!["shell", "read_file"]);
        assert!(request.default);
        assert_eq!(request.body, "Updated agent body");
    }

    #[test]
    fn test_update_agent_response_serialization() {
        let response = UpdateAgentResponse {
            name: "test-agent".to_string(),
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["name"], "test-agent");
    }
}
