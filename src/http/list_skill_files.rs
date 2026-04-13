use axum::{
    Json,
    extract::Path,
};

use crate::{
    BabataResult,
    error::BabataError,
    skill::{skill_dir, skill_exists},
};

use super::file_browser::{FileEntry, read_directory_recursive};

/// Handle GET /api/skills/{name}/files
pub(super) async fn handle(Path(name): Path<String>) -> BabataResult<Json<Vec<FileEntry>>> {
    if !skill_exists(&name)? {
        return Err(BabataError::not_found(format!("Skill '{}' not found", name)));
    }

    let skill_dir = skill_dir(&name)?;
    let files = read_directory_recursive(&skill_dir)
        .await
        .map_err(|err| BabataError::invalid_input(format!("Failed to read directory: {}", err)))?;

    Ok(Json(files))
}
