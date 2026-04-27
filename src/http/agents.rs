use axum::{
    Json,
    extract::{Path, Request, State},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use tower_http::services::ServeDir;

use crate::{
    BabataResult,
    agent::{
        AgentFrontmatter, agent_exists, delete_agent, load_agent_by_name, load_agents, save_agent,
    },
    error::BabataError,
    provider::ProviderConfig,
    utils::agent_dir,
};

use super::{
    HttpApp,
    file_browser::{BrowsedPath, FileEntry, browse_path, build_file_request},
    require_non_empty,
};

pub(super) async fn list() -> BabataResult<Json<ListAgentsResponse>> {
    let agents = load_agents()?;
    Ok(Json(ListAgentsResponse::from_agents(agents)))
}

pub(super) async fn get(Path(name): Path<String>) -> BabataResult<Json<GetAgentResponse>> {
    let agent = load_agent_by_name(&name).map_err(|err| {
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

pub(super) async fn create(
    State(_state): State<HttpApp>,
    Json(request): Json<CreateAgentRequest>,
) -> BabataResult<()> {
    require_non_empty(&request.name, "name")?;

    validate_provider_model_selection(&request.provider, &request.model)?;

    if agent_exists(&request.name) {
        return Err(BabataError::invalid_input(format!(
            "Agent '{}' already exists",
            request.name
        )));
    }

    if request.default
        && load_agents()?
            .values()
            .any(|agent| agent.frontmatter.default == Some(true))
    {
        return Err(BabataError::invalid_input(
            "Another default agent already exists. Only one agent can be default.",
        ));
    }

    let frontmatter = AgentFrontmatter {
        name: request.name.clone(),
        description: request.description,
        provider: request.provider,
        model: request.model,
        allowed_tools: request.allowed_tools,
        default: if request.default { Some(true) } else { None },
    };

    save_agent(&frontmatter, &request.body)?;
    Ok(())
}

pub(super) async fn update(
    Path(name): Path<String>,
    Json(request): Json<UpdateAgentRequest>,
) -> BabataResult<()> {
    if !agent_exists(&name) {
        return Err(BabataError::not_found(format!(
            "Agent '{}' not found",
            name
        )));
    }

    validate_provider_model_selection(&request.provider, &request.model)?;

    let existing_agent = load_agent_by_name(&name)?;
    let old_default = existing_agent.frontmatter.default == Some(true);
    let new_default = request.default;

    if new_default && !old_default {
        unset_current_default_agent(&name)?;
    }

    let frontmatter = AgentFrontmatter {
        name: name.clone(),
        description: request.description,
        provider: request.provider,
        model: request.model,
        allowed_tools: request.allowed_tools,
        default: Some(new_default),
    };

    save_agent(&frontmatter, &request.body)?;
    Ok(())
}

pub(super) async fn delete(Path(name): Path<String>) -> BabataResult<()> {
    if !agent_exists(&name) {
        return Err(BabataError::not_found(format!(
            "Agent '{}' not found",
            name
        )));
    }

    let agents = load_agents()?;
    if let Some(agent) = agents.get(&name) {
        let is_default = agent.frontmatter.default == Some(true);
        let is_only_agent = agents.len() == 1;

        if is_default && is_only_agent {
            return Err(BabataError::invalid_input(
                "Cannot delete the only default agent. System must have at least one agent.",
            ));
        }
    }

    delete_agent(&name)?;
    Ok(())
}

pub(super) async fn list_files(Path(name): Path<String>) -> BabataResult<Json<Vec<FileEntry>>> {
    if !agent_exists(&name) {
        return Err(BabataError::not_found(format!(
            "Agent '{}' not found",
            name
        )));
    }

    match browse_path(&agent_dir(&name)?, None).await? {
        BrowsedPath::Directory(entries) => Ok(Json(entries)),
        BrowsedPath::File(_) => {
            unreachable!("agent root path should always resolve to a directory")
        }
    }
}

pub(super) async fn get_file(
    Path((name, file_path)): Path<(String, String)>,
    request: Request,
) -> Response {
    match get_file_inner(&name, &file_path, request).await {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

async fn get_file_inner(name: &str, file_path: &str, request: Request) -> BabataResult<Response> {
    if !agent_exists(name) {
        return Err(BabataError::not_found(format!(
            "Agent '{}' not found",
            name
        )));
    }

    let base_dir = agent_dir(name)?;
    match browse_path(&base_dir, Some(file_path)).await? {
        BrowsedPath::Directory(entries) => Ok(Json(entries).into_response()),
        BrowsedPath::File(sanitized_path) => {
            let forwarded_request = build_file_request(request, &sanitized_path)?;
            let mut service = ServeDir::new(base_dir).append_index_html_on_directories(false);
            service
                .try_call(forwarded_request)
                .await
                .map(IntoResponse::into_response)
                .map_err(|err| BabataError::internal(format!("Failed to serve agent file: {err}")))
        }
    }
}

fn unset_current_default_agent(excluding: &str) -> BabataResult<()> {
    let agents = load_agents()?;

    for (agent_name, agent) in agents.iter() {
        if agent_name == excluding {
            continue;
        }

        if agent.frontmatter.default == Some(true) {
            let mut new_frontmatter = agent.frontmatter.clone();
            new_frontmatter.default = Some(false);
            save_agent(&new_frontmatter, &agent.body)?;
            log::info!("Unset default agent '{}'", agent_name);
            break;
        }
    }

    Ok(())
}

fn validate_provider_model_selection(provider_name: &str, model_id: &str) -> BabataResult<()> {
    let provider = ProviderConfig::load(provider_name)?;
    provider.find_model(model_id)?;
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
pub(crate) struct GetAgentResponse {
    pub name: String,
    pub description: String,
    pub provider: String,
    pub model: String,
    pub allowed_tools: Vec<String>,
    pub default: bool,
    pub body: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ListAgentsResponse {
    pub agents: Vec<AgentFrontmatter>,
}

impl ListAgentsResponse {
    fn from_agents(
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

#[cfg(test)]
mod tests {
    use super::UpdateAgentRequest;
    use serde_json::json;

    #[test]
    fn update_agent_request_deserialization() {
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
}
