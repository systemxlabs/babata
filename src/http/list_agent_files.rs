use axum::{Json, extract::Path};

use crate::{
    BabataResult,
    agent::{agent_dir, agent_exists},
    error::BabataError,
};

use super::file_browser::{FileEntry, read_directory_recursive};

/// Handle GET /api/agents/{name}/files
pub(super) async fn handle(Path(name): Path<String>) -> BabataResult<Json<Vec<FileEntry>>> {
    if !agent_exists(&name) {
        return Err(BabataError::not_found(format!(
            "Agent '{}' not found",
            name
        )));
    }

    let agent_dir = agent_dir(&name)?;
    let files = read_directory_recursive(&agent_dir)
        .await
        .map_err(|err| BabataError::invalid_input(format!("Failed to read directory: {}", err)))?;

    Ok(Json(files))
}
