use axum::{
    Json,
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ApiError, HttpApp};

/// Query parameters for listing task files
#[derive(Debug, Deserialize)]
pub(super) struct ListTaskFilesQuery {
    /// Optional subpath within the task directory
    #[serde(default)]
    path: Option<String>,
}

/// File or directory entry
#[derive(Debug, Serialize)]
pub(crate) struct FileEntry {
    /// Entry name
    pub(crate) name: String,
    /// Relative path from task directory
    pub(crate) path: String,
    /// Whether this is a directory
    pub(crate) is_dir: bool,
    /// File size in bytes (None for directories)
    pub(crate) size: Option<u64>,
    /// Last modified timestamp in seconds since Unix epoch
    pub(crate) modified: Option<u64>,
}

/// Response for listing task files
#[derive(Debug, Serialize)]
pub(crate) struct ListTaskFilesResponse {
    /// Task ID
    pub(crate) task_id: String,
    /// Current path relative to task directory
    pub(crate) current_path: String,
    /// Directory entries
    pub(crate) entries: Vec<FileEntry>,
}

/// Handle GET /tasks/{task_id}/files
pub(super) async fn handle(
    State(state): State<HttpApp>,
    Path(task_id): Path<String>,
    Query(query): Query<ListTaskFilesQuery>,
) -> Response {
    // Parse task ID
    let task_id = match Uuid::parse_str(&task_id) {
        Ok(task_id) => task_id,
        Err(err) => {
            return ApiError::bad_request(format!("Invalid task id '{}': {}", task_id, err))
                .into_response();
        }
    };

    // Verify task exists
    if let Err(err) = state.task_manager.get_task(task_id) {
        return ApiError::from_babata_error(err).into_response();
    }

    // Get task directory path
    let task_dir = match crate::utils::babata_dir() {
        Ok(babata_dir) => babata_dir.join("tasks").join(task_id.to_string()),
        Err(err) => return ApiError::from_babata_error(err).into_response(),
    };

    // Parse requested subpath
    let sub_path = query.path.as_deref().unwrap_or("");
    let target_path = task_dir.join(sub_path);

    // Security check: ensure path is within task directory
    if !is_path_within(&target_path, &task_dir) {
        return ApiError::bad_request("Invalid path: path traversal detected".to_string())
            .into_response();
    }

    // Check if path exists
    if !target_path.exists() {
        return ApiError::bad_request(format!("Path not found: {}", sub_path)).into_response();
    }

    // Ensure it's a directory
    if !target_path.is_dir() {
        return ApiError::bad_request("Path is not a directory".to_string()).into_response();
    }

    // Read directory contents
    match read_directory(&target_path, &task_dir).await {
        Ok(entries) => {
            let response = ListTaskFilesResponse {
                task_id: task_id.to_string(),
                current_path: sub_path.to_string(),
                entries,
            };
            Json(response).into_response()
        }
        Err(err) => {
            ApiError::bad_request(format!("Failed to read directory: {}", err)).into_response()
        }
    }
}

/// Security check: ensure child path is within parent path
fn is_path_within(child: &std::path::Path, parent: &std::path::Path) -> bool {
    // Normalize paths for comparison
    let parent_canonical = parent.canonicalize();
    let child_canonical = child.canonicalize();

    match (parent_canonical, child_canonical) {
        (Ok(parent), Ok(child)) => child.starts_with(&parent),
        // If canonicalization fails, reject the path
        // This is safer than trying to resolve non-existent paths
        _ => false,
    }
}

/// Read directory and return file entries
async fn read_directory(
    dir: &std::path::Path,
    base_dir: &std::path::Path,
) -> Result<Vec<FileEntry>, std::io::Error> {
    let mut entries = Vec::new();
    let mut dir_entries = tokio::fs::read_dir(dir).await?;

    while let Some(entry) = dir_entries.next_entry().await? {
        let metadata = entry.metadata().await?;
        let name = entry.file_name().to_string_lossy().to_string();

        // Calculate relative path
        let full_path = entry.path();
        let rel_path = full_path
            .strip_prefix(base_dir)
            .unwrap_or(&full_path)
            .to_string_lossy()
            .to_string()
            .replace('\\', "/");

        entries.push(FileEntry {
            name,
            path: rel_path,
            is_dir: metadata.is_dir(),
            size: if metadata.is_file() {
                Some(metadata.len())
            } else {
                None
            },
            modified: metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs()),
        });
    }

    // Sort: directories first, then files, both alphabetically
    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_path_within_valid() {
        // Use std::env::current_dir() which should exist
        let base = std::env::current_dir().unwrap();
        let child = base.join("src").join("main.rs");

        assert!(is_path_within(&child, &base));
    }

    #[test]
    fn test_is_path_within_traversal_attack() {
        // Use std::env::current_dir() which should exist
        let base = std::env::current_dir().unwrap();
        // Path traversal: going up and then to /etc/passwd
        let child = base.join("..").join("..").join("etc").join("passwd");

        assert!(!is_path_within(&child, &base));
    }

    #[test]
    fn test_is_path_within_different_branch() {
        // Use std::env::current_dir() which should exist
        let base = std::env::current_dir().unwrap();
        let base_parent = base.parent().unwrap();
        let child = base_parent.join("other-project").join("file.txt");

        assert!(!is_path_within(&child, &base));
    }

    #[test]
    fn test_is_path_within_nonexistent_parent() {
        let base = std::path::Path::new("/nonexistent/path/to/task-123");
        let child = base.join("file.txt");

        // Should return false when parent doesn't exist
        assert!(!is_path_within(&child, base));
    }
}
