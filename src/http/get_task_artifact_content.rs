use std::{
    fs,
    path::{Path, PathBuf},
};

use axum::{
    Json,
    extract::{Path as AxumPath, Query, State},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ApiError, HttpApp};

const MAX_PREVIEW_BYTES: usize = 16 * 1024;

pub(super) async fn handle(
    State(state): State<HttpApp>,
    AxumPath(task_id): AxumPath<String>,
    Query(query): Query<ArtifactContentQuery>,
) -> Response {
    let task_id = match parse_task_id(&task_id) {
        Ok(task_id) => task_id,
        Err(err) => return err.into_response(),
    };

    let artifact_root = match state.task_manager.task_artifact_root(task_id) {
        Ok(artifact_root) => artifact_root,
        Err(err) => return ApiError::from_babata_error(err).into_response(),
    };

    if !artifact_root.exists() {
        return Json(unsupported_response(
            task_id,
            query.path,
            0,
            "Artifact path was not found",
        ))
        .into_response();
    }

    let artifact_path = match resolve_artifact_path(&artifact_root, &query.path) {
        Ok(artifact_path) => artifact_path,
        Err(err) => return err.into_response(),
    };

    let target_check = match validate_preview_target(&artifact_root, &artifact_path) {
        Ok(target_check) => target_check,
        Err(err) => return err.into_response(),
    };
    if let Some(reason) = target_check.reason {
        return Json(unsupported_response(
            task_id,
            query.path,
            target_check.size_bytes,
            &reason,
        ))
        .into_response();
    }

    let metadata = match fs::metadata(&artifact_path) {
        Ok(metadata) => metadata,
        Err(err) => {
            return Json(unsupported_response(
                task_id,
                query.path,
                0,
                &format!("Artifact path was not found: {err}"),
            ))
            .into_response();
        }
    };

    if metadata.is_dir() {
        return Json(ArtifactContentResponse {
            task_id: task_id.to_string(),
            path: query.path,
            is_text: false,
            size_bytes: metadata.len(),
            content: None,
            reason: Some("Artifact path points to a directory".to_string()),
        })
        .into_response();
    }

    let bytes = match fs::read(&artifact_path) {
        Ok(bytes) => bytes,
        Err(err) => {
            return Json(unsupported_response(
                task_id,
                query.path,
                metadata.len(),
                &format!("Failed to read artifact preview: {err}"),
            ))
            .into_response();
        }
    };

    let (is_text, content, reason) = match String::from_utf8(bytes) {
        Ok(text) => {
            if text.len() <= MAX_PREVIEW_BYTES {
                (true, Some(text), None)
            } else {
                (
                    true,
                    Some(truncate_utf8(&text, MAX_PREVIEW_BYTES)),
                    Some(format!(
                        "Content truncated to {} bytes for preview",
                        MAX_PREVIEW_BYTES
                    )),
                )
            }
        }
        Err(_) => (
            false,
            None,
            Some("Artifact is not UTF-8 text; preview is unsupported".to_string()),
        ),
    };

    Json(ArtifactContentResponse {
        task_id: task_id.to_string(),
        path: query.path,
        is_text,
        size_bytes: metadata.len(),
        content,
        reason,
    })
    .into_response()
}

fn parse_task_id(task_id: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(task_id)
        .map_err(|err| ApiError::bad_request(format!("Invalid task id '{}': {}", task_id, err)))
}

fn resolve_artifact_path(artifact_root: &Path, requested_path: &str) -> Result<PathBuf, ApiError> {
    let requested = requested_path.trim();
    if requested.is_empty() {
        return Err(ApiError::bad_request("Artifact path cannot be empty"));
    }

    let path = Path::new(requested);
    if path.is_absolute() {
        return Err(ApiError::bad_request(format!(
            "Artifact path '{}' must be relative",
            requested_path
        )));
    }

    if path
        .components()
        .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(ApiError::bad_request(format!(
            "Artifact path '{}' contains unsupported path segments",
            requested_path
        )));
    }

    Ok(artifact_root.join(path))
}

fn validate_preview_target(
    artifact_root: &Path,
    artifact_path: &Path,
) -> Result<PreviewTargetCheck, ApiError> {
    let symlink_metadata = match fs::symlink_metadata(artifact_path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(PreviewTargetCheck {
                size_bytes: 0,
                reason: Some("Artifact path was not found".to_string()),
            });
        }
        Err(err) => {
            return Err(ApiError::bad_request(format!(
                "Failed to inspect artifact '{}': {}",
                artifact_path.display(),
                err
            )));
        }
    };

    if symlink_metadata.file_type().is_symlink() {
        return Ok(PreviewTargetCheck {
            size_bytes: 0,
            reason: Some("Artifact preview does not allow symlink targets".to_string()),
        });
    }

    let canonical_root = artifact_root.canonicalize().map_err(|err| {
        ApiError::bad_request(format!(
            "Failed to resolve artifact root '{}': {}",
            artifact_root.display(),
            err
        ))
    })?;
    let canonical_target = artifact_path.canonicalize().map_err(|err| {
        ApiError::bad_request(format!(
            "Failed to resolve artifact '{}': {}",
            artifact_path.display(),
            err
        ))
    })?;

    if !canonical_target.starts_with(&canonical_root) {
        return Ok(PreviewTargetCheck {
            size_bytes: symlink_metadata.len(),
            reason: Some("Artifact path resolves outside the artifact root".to_string()),
        });
    }

    Ok(PreviewTargetCheck {
        size_bytes: symlink_metadata.len(),
        reason: None,
    })
}

fn truncate_utf8(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }

    let mut end = max_bytes;
    while !text.is_char_boundary(end) {
        end -= 1;
    }

    text[..end].to_string()
}

fn unsupported_response(
    task_id: Uuid,
    path: String,
    size_bytes: u64,
    reason: &str,
) -> ArtifactContentResponse {
    ArtifactContentResponse {
        task_id: task_id.to_string(),
        path,
        is_text: false,
        size_bytes,
        content: None,
        reason: Some(reason.to_string()),
    }
}

struct PreviewTargetCheck {
    size_bytes: u64,
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ArtifactContentQuery {
    path: String,
}

#[derive(Debug, Serialize)]
struct ArtifactContentResponse {
    task_id: String,
    path: String,
    is_text: bool,
    size_bytes: u64,
    content: Option<String>,
    reason: Option<String>,
}
