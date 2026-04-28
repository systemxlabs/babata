use std::path::Path;

use axum::{body::Body, extract::Request, http::Uri};
use serde::Serialize;

use crate::{BabataResult, error::BabataError, utils::SKIP_DIRS};

/// File or directory entry
#[derive(Debug, Serialize)]
pub(crate) struct FileEntry {
    /// Entry name
    pub(crate) name: String,
    /// Relative path from the base directory
    pub(crate) path: String,
    /// Whether this is a directory
    pub(crate) is_dir: bool,
    /// File size in bytes (None for directories)
    pub(crate) size: Option<u64>,
    /// Last modified timestamp in seconds since Unix epoch
    pub(crate) modified: Option<u64>,
}

pub(crate) enum BrowsedPath {
    Directory(Vec<FileEntry>),
    File(String),
}

pub(crate) async fn browse_path(
    base_dir: &Path,
    relative_path: Option<&str>,
) -> BabataResult<BrowsedPath> {
    let sanitized_path = relative_path
        .unwrap_or_default()
        .trim_start_matches('/')
        .replace('\\', "/");

    let target_path = if sanitized_path.is_empty() {
        base_dir.to_path_buf()
    } else {
        base_dir.join(&sanitized_path)
    };

    match tokio::fs::metadata(&target_path).await {
        Ok(metadata) if metadata.is_dir() => {
            let entries = read_directory(
                base_dir,
                if sanitized_path.is_empty() {
                    None
                } else {
                    Some(sanitized_path.as_str())
                },
            )
            .await
            .map_err(|err| {
                BabataError::invalid_input(format!("Failed to read directory: {}", err))
            })?;
            Ok(BrowsedPath::Directory(entries))
        }
        Ok(metadata) if metadata.is_file() => Ok(BrowsedPath::File(sanitized_path)),
        _ => Err(BabataError::not_found(format!(
            "Path '{}' not found",
            relative_path.unwrap_or_default()
        ))),
    }
}

/// Read a single directory and return its entries (non-recursive).
pub(crate) async fn read_directory(
    base_dir: &Path,
    relative_path: Option<&str>,
) -> Result<Vec<FileEntry>, std::io::Error> {
    let target_dir = if let Some(rel) = relative_path {
        base_dir.join(rel)
    } else {
        base_dir.to_path_buf()
    };

    let mut entries = Vec::new();
    let mut dir_entries = tokio::fs::read_dir(&target_dir).await?;

    while let Some(entry) = dir_entries.next_entry().await? {
        let metadata = entry.metadata().await?;
        let name = entry.file_name().to_string_lossy().to_string();
        let full_path = entry.path();
        let rel_path = full_path
            .strip_prefix(base_dir)
            .unwrap_or(&full_path)
            .to_string_lossy()
            .to_string()
            .replace('\\', "/");
        let is_dir = metadata.is_dir();

        // Skip directories in the skip list
        if is_dir && SKIP_DIRS.contains(&name.as_str()) {
            continue;
        }

        entries.push(FileEntry {
            name,
            path: rel_path,
            is_dir,
            size: if is_dir { None } else { Some(metadata.len()) },
            modified: metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs()),
        });
    }

    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.path.cmp(&b.path),
    });

    Ok(entries)
}

pub(crate) fn build_file_request(request: Request, file_path: &str) -> BabataResult<Request> {
    let method = request.method().clone();
    let version = request.version();
    let headers = request.headers().clone();
    let sanitized_path = file_path.trim_start_matches('/').replace('\\', "/");
    let forwarded_uri: Uri = format!("/{}", sanitized_path).parse().map_err(|err| {
        BabataError::invalid_input(format!("Invalid file path '{}': {}", file_path, err))
    })?;

    let mut forwarded_request = Request::builder()
        .method(method)
        .uri(forwarded_uri)
        .version(version)
        .body(Body::empty())
        .map_err(|_| BabataError::internal("Failed to build forwarded file request"))?;
    *forwarded_request.headers_mut() = headers;

    Ok(forwarded_request)
}

#[cfg(test)]
mod tests {
    use super::{BrowsedPath, browse_path, build_file_request, read_directory};
    use std::path::PathBuf;

    use axum::{
        body::Body,
        extract::Request,
        http::{Method, Version, header},
    };

    #[test]
    fn rewrites_api_request_to_relative_uri() {
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/tasks/task-id/files/logs/output.txt")
            .version(Version::HTTP_11)
            .header(header::ACCEPT, "text/plain")
            .body(Body::empty())
            .expect("request");

        let forwarded = build_file_request(request, "logs/output.txt").expect("forwarded");

        assert_eq!(forwarded.uri().path(), "/logs/output.txt");
        assert_eq!(forwarded.method(), Method::GET);
        assert_eq!(forwarded.version(), Version::HTTP_11);
        assert_eq!(forwarded.headers()[header::ACCEPT], "text/plain");
    }

    #[test]
    fn normalizes_windows_separators_in_forwarded_uri() {
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/tasks/task-id/files/logs\\output.txt")
            .body(Body::empty())
            .expect("request");

        let forwarded = build_file_request(request, "logs\\output.txt").expect("forwarded");

        assert_eq!(forwarded.uri().path(), "/logs/output.txt");
    }

    #[tokio::test]
    async fn reads_single_directory_non_recursively() {
        let base = std::env::current_dir().expect("current dir").join("src");
        if base.exists() {
            let entries = read_directory(&base, None).await.expect("entries");
            assert!(!entries.is_empty());
            for entry in &entries {
                assert!(!entry.path.starts_with('/'));
                assert!(!entry.path.starts_with('\\'));
                // Should not contain nested paths (no slashes for direct children)
                assert!(!entry.path.contains('/'));
            }
        }
    }

    #[tokio::test]
    async fn reads_subdirectory_with_relative_path() {
        let base = std::env::current_dir().expect("current dir");
        let src = base.join("src");
        if src.exists() {
            let entries = read_directory(&base, Some("src")).await.expect("entries");
            assert!(!entries.is_empty());
            for entry in &entries {
                assert!(entry.path.starts_with("src/"));
            }
        }
    }

    #[tokio::test]
    async fn returns_error_for_missing_directory() {
        let base = PathBuf::from("/nonexistent/path/xyz123");
        let result = read_directory(&base, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn returns_error_for_missing_subdirectory() {
        let base = std::env::current_dir().expect("current dir");
        let result = read_directory(&base, Some("nonexistent_dir_xyz")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn browse_path_lists_root_directory_when_relative_path_is_empty() {
        let base = std::env::current_dir().expect("current dir");
        let browsed = browse_path(&base, None).await.expect("browse root");

        assert!(matches!(browsed, BrowsedPath::Directory(_)));
    }
}
