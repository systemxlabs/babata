use axum::{
    Json,
    extract::{Path, State},
};
use serde::Serialize;

use crate::{BabataResult, error::BabataError};

use super::{HttpApp, ensure_task_exists, parse_task_id};

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

/// Handle GET /api/tasks/{task_id}/files
pub(super) async fn handle(
    State(state): State<HttpApp>,
    Path(task_id): Path<String>,
) -> BabataResult<Json<Vec<FileEntry>>> {
    let task_id = parse_task_id(&task_id)?;
    ensure_task_exists(&state.task_manager, task_id)?;

    let task_dir = crate::utils::babata_dir()
        .map_err(BabataError::from)?
        .join("tasks")
        .join(task_id.to_string());

    if !task_dir.exists() {
        return Ok(Json(Vec::new()));
    }

    let files = read_directory_recursive(&task_dir)
        .await
        .map_err(|err| BabataError::invalid_input(format!("Failed to read directory: {}", err)))?;

    Ok(Json(files))
}

/// Recursively read directory and return all file entries using iterative approach
async fn read_directory_recursive(
    base_dir: &std::path::Path,
) -> Result<Vec<FileEntry>, std::io::Error> {
    let mut entries = Vec::new();
    let mut dirs_to_process: Vec<std::path::PathBuf> = vec![base_dir.to_path_buf()];

    while let Some(current_dir) = dirs_to_process.pop() {
        let mut dir_entries = tokio::fs::read_dir(&current_dir).await?;

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

            let is_dir = metadata.is_dir();

            entries.push(FileEntry {
                name,
                path: rel_path,
                is_dir,
                size: if is_dir { None } else { Some(metadata.len()) },
                modified: metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs()),
            });

            // Add subdirectory to processing queue
            if is_dir {
                dirs_to_process.push(full_path);
            }
        }
    }

    // Sort: directories first, then files, both alphabetically by path
    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.path.cmp(&b.path),
    });

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_directory_recursive() {
        // Test with current directory's src folder
        let base = std::env::current_dir().unwrap().join("src");
        if base.exists() {
            let entries = read_directory_recursive(&base).await.unwrap();
            // Should find some files
            assert!(!entries.is_empty());
            // All entries should have paths starting from base
            for entry in &entries {
                assert!(!entry.path.starts_with('/'));
                assert!(!entry.path.starts_with('\\'));
            }
        }
    }

    #[tokio::test]
    async fn test_read_directory_recursive_empty() {
        // Test with a non-existent directory
        let base = std::path::PathBuf::from("/nonexistent/path/xyz123");
        let result = read_directory_recursive(&base).await;
        // Should return an error for non-existent directory
        assert!(result.is_err());
    }
}
