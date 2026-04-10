use axum::extract::Path;

use crate::{
    BabataResult,
    skill::{delete_skill, skill_exists},
};

pub(super) async fn handle(Path(name): Path<String>) -> BabataResult<()> {
    // Check if skill exists
    if !skill_exists(&name)? {
        return Err(crate::error::BabataError::not_found(format!(
            "Skill '{}' not found",
            name
        )));
    }

    delete_skill(&name)?;
    Ok(())
}
