use std::{
    fs,
    path::{Path, PathBuf},
};

use axum::{
    Json,
    extract::{Path as AxumPath, State},
    response::{IntoResponse, Response},
};
use serde::Serialize;
use uuid::Uuid;

use super::{ApiError, HttpApp};

pub(super) async fn handle(
    State(state): State<HttpApp>,
    AxumPath(task_id): AxumPath<String>,
) -> Response {
    let task_id = match parse_task_id(&task_id) {
        Ok(task_id) => task_id,
        Err(err) => return err.into_response(),
    };

    let artifact_root = match state.task_manager.task_artifact_root(task_id) {
        Ok(artifact_root) => artifact_root,
        Err(err) => return ApiError::from_babata_error(err).into_response(),
    };

    let artifacts = match list_artifacts(&artifact_root) {
        Ok(artifacts) => artifacts,
        Err(err) => return err.into_response(),
    };

    Json(TaskArtifactsResponse {
        task_id: task_id.to_string(),
        artifacts,
    })
    .into_response()
}

fn parse_task_id(task_id: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(task_id)
        .map_err(|err| ApiError::bad_request(format!("Invalid task id '{}': {}", task_id, err)))
}

fn list_artifacts(artifact_root: &Path) -> Result<Vec<ArtifactResponse>, ApiError> {
    if !artifact_root.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    collect_artifacts(artifact_root, artifact_root, &mut entries)?;
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

fn collect_artifacts(
    artifact_root: &Path,
    current_dir: &Path,
    entries: &mut Vec<ArtifactResponse>,
) -> Result<(), ApiError> {
    let read_dir = fs::read_dir(current_dir).map_err(|err| {
        ApiError::bad_request(format!(
            "Failed to read artifacts directory '{}': {}",
            current_dir.display(),
            err
        ))
    })?;

    for entry in read_dir {
        let entry = entry.map_err(|err| {
            ApiError::bad_request(format!(
                "Failed to read artifact entry in '{}': {}",
                current_dir.display(),
                err
            ))
        })?;
        let path = entry.path();
        let symlink_metadata = fs::symlink_metadata(&path).map_err(|err| {
            ApiError::bad_request(format!(
                "Failed to read artifact metadata '{}': {}",
                path.display(),
                err
            ))
        })?;
        if symlink_metadata.file_type().is_symlink() {
            continue;
        }
        let metadata = entry.metadata().map_err(|err| {
            ApiError::bad_request(format!(
                "Failed to read artifact metadata '{}': {}",
                path.display(),
                err
            ))
        })?;

        if metadata.is_dir() {
            collect_artifacts(artifact_root, &path, entries)?;
            continue;
        }

        let relative_path = normalize_relative_path(relative_path(artifact_root, &path)?);
        entries.push(ArtifactResponse {
            path: relative_path,
            size_bytes: metadata.len(),
            is_text: is_text_file(&path),
        });
    }

    Ok(())
}

fn relative_path(base: &Path, path: &Path) -> Result<PathBuf, ApiError> {
    path.strip_prefix(base)
        .map(Path::to_path_buf)
        .map_err(|err| {
            ApiError::bad_request(format!(
                "Failed to strip artifact root '{}' from '{}': {}",
                base.display(),
                path.display(),
                err
            ))
        })
}

fn normalize_relative_path(path: PathBuf) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/")
}

fn is_text_file(path: &Path) -> bool {
    fs::read(path)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .is_some()
}

#[derive(Debug, Serialize)]
struct TaskArtifactsResponse {
    task_id: String,
    artifacts: Vec<ArtifactResponse>,
}

#[derive(Debug, Serialize)]
struct ArtifactResponse {
    path: String,
    size_bytes: u64,
    is_text: bool,
}
