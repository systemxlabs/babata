use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use serde::{Deserialize, Serialize};

use crate::{
    BabataResult,
    error::BabataError,
    utils::{agent_dir, agents_dir},
};
use log::error;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AgentFrontmatter {
    pub name: String,
    pub description: String,
    pub provider: String,
    pub model: String,
    pub allowed_tools: Vec<String>,
    pub default: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct Agent {
    pub path: PathBuf,
    pub frontmatter: AgentFrontmatter,
    pub body: String,
}

impl Agent {
    pub fn home(&self) -> BabataResult<PathBuf> {
        let home = self
            .path
            .parent()
            .ok_or_else(|| BabataError::config("Invalid agent path"))?
            .to_path_buf();
        Ok(home)
    }
}

pub fn load_agents() -> BabataResult<HashMap<String, Arc<Agent>>> {
    load_agents_from_dir(&agents_dir()?)
}

fn load_agents_from_dir(dir: &Path) -> BabataResult<HashMap<String, Arc<Agent>>> {
    if !dir.exists() {
        return Ok(HashMap::new());
    }

    if !dir.is_dir() {
        return Err(BabataError::config(format!(
            "Agents path '{}' is not a directory",
            dir.display()
        )));
    }

    let mut agents = HashMap::new();
    let entries = std::fs::read_dir(dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let Some(agent_name) = path.file_name().and_then(|name| name.to_str()) else {
            return Err(BabataError::config(format!(
                "Agent directory '{}' is not valid UTF-8",
                path.display()
            )));
        };
        let agent = match load_agent_by_name(agent_name) {
            Ok(agent) => agent,
            Err(e) => {
                error!("Failed to load agent {}: {}", agent_name, e);
                continue;
            }
        };

        agents.insert(agent_name.to_string(), agent);
    }

    // Validate exactly one default agent
    let default_count = agents
        .values()
        .filter(|d| d.frontmatter.default == Some(true))
        .count();
    if agents.is_empty() && default_count == 0 {
        return Err(BabataError::config(
            "No default agent found. Exactly one agent must have 'default: true' in its frontmatter.",
        ));
    }
    if default_count > 1 {
        return Err(BabataError::config(format!(
            "Multiple default agents found ({}). Exactly one agent must have 'default: true' in its frontmatter.",
            default_count
        )));
    }

    Ok(agents)
}

fn parse_agent_content(content: &str, path: &Path) -> BabataResult<(AgentFrontmatter, String)> {
    let mut lines = content.lines();
    let Some(first) = lines.next() else {
        return Err(BabataError::config(format!(
            "Agent file '{}' is empty or missing headers",
            path.display()
        )));
    };
    if first != "---" {
        return Err(BabataError::config(format!(
            "Agent file '{}' is missing yaml headers (expected starting '---')",
            path.display()
        )));
    }

    let mut header_lines = Vec::new();
    let mut body_lines = Vec::new();
    let mut in_headers = true;

    for line in lines {
        if in_headers {
            if line == "---" {
                in_headers = false;
                continue;
            }
            header_lines.push(line);
        } else {
            body_lines.push(line);
        }
    }

    if in_headers {
        return Err(BabataError::config(format!(
            "Agent file '{}' starts with '---' but has no closing '---'",
            path.display()
        )));
    }

    let header_raw = header_lines.join("\n");
    let body = body_lines.join("\n");
    let headers = serde_yaml::from_str::<AgentFrontmatter>(&header_raw).map_err(|err| {
        BabataError::config(format!(
            "Failed to parse agent headers in '{}': {}",
            path.display(),
            err
        ))
    })?;

    Ok((headers, body))
}

/// Save an agent to AGENT.md file
pub fn save_agent(frontmatter: &AgentFrontmatter, body: &str) -> BabataResult<()> {
    let agent_dir = agent_dir(&frontmatter.name)?;
    std::fs::create_dir_all(&agent_dir)?;

    let agent_path = agent_dir.join("AGENT.md");

    let frontmatter_yaml = serde_yaml::to_string(frontmatter).map_err(|err| {
        BabataError::config(format!("Failed to serialize agent frontmatter: {}", err))
    })?;

    let content = format!("---\n{}---\n{}", frontmatter_yaml, body);

    std::fs::write(&agent_path, content)?;

    Ok(())
}

/// Delete an agent by name (removes the entire agent directory)
pub fn delete_agent(name: &str) -> BabataResult<()> {
    let agent_dir = agent_dir(name)?;

    if !agent_dir.exists() {
        return Err(BabataError::config(format!(
            "Agent '{}' does not exist",
            name
        )));
    }

    std::fs::remove_dir_all(&agent_dir)?;

    Ok(())
}

/// Check if an agent exists
pub fn agent_exists(name: &str) -> bool {
    match agent_dir(name) {
        Ok(dir) => {
            let agent_path = dir.join("AGENT.md");
            agent_path.is_file()
        }
        Err(_) => false,
    }
}

/// Load a single agent by name
pub fn load_agent_by_name(name: &str) -> BabataResult<Arc<Agent>> {
    let agent_path = agent_dir(name)?.join("AGENT.md");

    if !agent_path.is_file() {
        return Err(BabataError::config(format!(
            "Agent '{}' not found at '{}'",
            name,
            agent_path.display()
        )));
    }

    let content = std::fs::read_to_string(&agent_path)?;

    let (frontmatter, body) = parse_agent_content(&content, &agent_path)?;

    let agent = Agent {
        path: agent_path,
        frontmatter,
        body,
    };

    Ok(Arc::new(agent))
}

pub fn load_default_agent() -> BabataResult<Arc<Agent>> {
    let agents_dir = agents_dir()?;
    if !agents_dir.exists() {
        return Err(BabataError::internal("No any agent found"));
    }

    if !agents_dir.is_dir() {
        return Err(BabataError::config(format!(
            "Agents path '{}' is not a directory",
            agents_dir.display()
        )));
    }

    let entries = std::fs::read_dir(&agents_dir)?;

    for entry in entries {
        let entry = entry.map_err(|err| {
            BabataError::config(format!(
                "Failed to read agents directory entry in '{}': {}",
                agents_dir.display(),
                err
            ))
        })?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let Some(agent_name) = path.file_name().and_then(|name| name.to_str()) else {
            return Err(BabataError::config(format!(
                "Agent directory '{}' is not valid UTF-8",
                path.display()
            )));
        };
        let agent = match load_agent_by_name(agent_name) {
            Ok(agent) => agent,
            Err(e) => {
                error!("Failed to load agent {}: {}", agent_name, e);
                continue;
            }
        };
        if matches!(agent.frontmatter.default, Some(true)) {
            return Ok(agent);
        }
    }

    Err(BabataError::internal("No default agent found"))
}
