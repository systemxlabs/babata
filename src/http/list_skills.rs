use axum::Json;
use serde::Serialize;

use crate::{
    BabataResult,
    skill::{SkillFrontmatter, load_skills},
};

pub(super) async fn handle() -> BabataResult<Json<ListSkillsResponse>> {
    let skills = load_skills()?;
    Ok(Json(ListSkillsResponse::from_skills(skills)))
}

#[derive(Debug, Serialize)]
pub(crate) struct ListSkillsResponse {
    pub skills: Vec<SkillFrontmatter>,
}

impl ListSkillsResponse {
    pub(crate) fn from_skills(skills: Vec<crate::skill::Skill>) -> Self {
        Self {
            skills: skills.into_iter().map(|skill| skill.frontmatter).collect(),
        }
    }
}
