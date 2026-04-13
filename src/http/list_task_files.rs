use axum::{
    Json,
    extract::{Path, State},
};

use crate::{BabataResult, error::BabataError, utils::task_dir};

use super::{
    HttpApp, ensure_task_exists,
    file_browser::{FileEntry, read_directory_recursive},
    parse_task_id,
};

/// Handle GET /api/tasks/{task_id}/files
pub(super) async fn handle(
    State(state): State<HttpApp>,
    Path(task_id): Path<String>,
) -> BabataResult<Json<Vec<FileEntry>>> {
    let task_id = parse_task_id(&task_id)?;
    ensure_task_exists(&state.task_manager, task_id)?;

    let task_dir = task_dir(task_id)?;

    if !task_dir.exists() {
        return Ok(Json(Vec::new()));
    }

    let files = read_directory_recursive(&task_dir)
        .await
        .map_err(|err| BabataError::invalid_input(format!("Failed to read directory: {}", err)))?;

    Ok(Json(files))
}
