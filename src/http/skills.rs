use axum::{
    Json,
    extract::{Path, Request},
    response::{IntoResponse, Response},
};
use serde::Serialize;
use tower_http::services::ServeDir;

use crate::{
    BabataResult,
    error::BabataError,
    skill::{SkillFrontmatter, delete_skill, load_skills, skill_dir, skill_exists},
};

use super::file_browser::{FileEntry, build_file_request, read_directory_recursive};

pub(super) async fn list() -> BabataResult<Json<ListSkillsResponse>> {
    let skills = load_skills()?;
    Ok(Json(ListSkillsResponse::from_skills(skills)))
}

pub(super) async fn delete(Path(name): Path<String>) -> BabataResult<()> {
    if !skill_exists(&name)? {
        return Err(BabataError::not_found(format!(
            "Skill '{}' not found",
            name
        )));
    }

    delete_skill(&name)?;
    Ok(())
}

pub(super) async fn list_files(Path(name): Path<String>) -> BabataResult<Json<Vec<FileEntry>>> {
    if !skill_exists(&name)? {
        return Err(BabataError::not_found(format!(
            "Skill '{}' not found",
            name
        )));
    }

    let files = read_directory_recursive(&skill_dir(&name)?)
        .await
        .map_err(|err| BabataError::invalid_input(format!("Failed to read directory: {}", err)))?;

    Ok(Json(files))
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
    if !skill_exists(name)? {
        return Err(BabataError::not_found(format!(
            "Skill '{}' not found",
            name
        )));
    }

    let forwarded_request = build_file_request(request, file_path)?;
    let mut service = ServeDir::new(skill_dir(name)?).append_index_html_on_directories(false);
    service
        .try_call(forwarded_request)
        .await
        .map(IntoResponse::into_response)
        .map_err(|err| BabataError::internal(format!("Failed to serve skill file: {err}")))
}

#[derive(Debug, Serialize)]
pub(crate) struct ListSkillsResponse {
    pub skills: Vec<SkillFrontmatter>,
}

impl ListSkillsResponse {
    fn from_skills(skills: Vec<crate::skill::Skill>) -> Self {
        Self {
            skills: skills.into_iter().map(|skill| skill.frontmatter).collect(),
        }
    }
}
