use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use serde::Serialize;
use uuid::Uuid;

use super::{ApiError, HttpApp};

/// Response for file metadata (JSON format)
#[derive(Debug, Serialize)]
pub(crate) struct FileMetadataResponse {
    /// File name
    pub(crate) name: String,
    /// Relative path from task directory
    pub(crate) path: String,
    /// File size in bytes
    pub(crate) size: u64,
    /// Last modified timestamp in seconds since Unix epoch
    pub(crate) modified: Option<u64>,
    /// MIME type
    pub(crate) content_type: String,
}

/// Handle GET /tasks/{task_id}/files/{*path}
pub(super) async fn handle(
    State(state): State<HttpApp>,
    Path((task_id, file_path)): Path<(String, String)>,
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

    // Resolve the file path (security: prevent directory traversal)
    let file_path = file_path.replace('/', std::path::MAIN_SEPARATOR_STR);
    let target_path = match normalize_path(&task_dir, &file_path) {
        Some(path) => path,
        None => {
            return ApiError::bad_request("Invalid file path: directory traversal detected")
                .into_response();
        }
    };

    // Check if path exists and is a file
    let metadata = match tokio::fs::metadata(&target_path).await {
        Ok(meta) => {
            if !meta.is_file() {
                return ApiError::bad_request(format!(
                    "'{}' is not a file (might be a directory)",
                    file_path
                ))
                .into_response();
            }
            meta
        }
        Err(err) => {
            return ApiError::bad_request(format!("File not found: {}", err)).into_response();
        }
    };

    // Get file name
    let name = target_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| file_path.clone());

    // Calculate relative path
    let rel_path = target_path
        .strip_prefix(&task_dir)
        .map(|p| p.to_string_lossy().to_string().replace('\\', "/"))
        .unwrap_or_else(|_| file_path.clone());

    // Detect content type
    let content_type = detect_content_type(&name, &target_path).await;

    // Check if it's a text file based on content type
    let is_text = content_type.starts_with("text/")
        || content_type == "application/json"
        || content_type == "application/javascript"
        || content_type == "application/xml"
        || content_type == "application/yaml"
        || content_type == "application/toml"
        || content_type == "application/x-sh"
        || content_type == "application/x-httpd-php"
        || content_type == "application/x-python-code"
        || content_type == "application/x-rust"
        || content_type.starts_with("application/x-");

    if is_text {
        // Read and return text content
        match tokio::fs::read_to_string(&target_path).await {
            Ok(content) => {
                let response = serde_json::json!({
                    "name": name,
                    "path": rel_path,
                    "size": metadata.len(),
                    "modified": metadata
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs()),
                    "content_type": content_type,
                    "content": content,
                    "encoding": "utf-8"
                });
                axum::Json(response).into_response()
            }
            Err(err) => {
                // Try to read as binary if UTF-8 fails
                match tokio::fs::read(&target_path).await {
                    Ok(bytes) => {
                        // Return base64 encoded content for binary files
                        let base64_content = base64::encode(&bytes);
                        let response = serde_json::json!({
                            "name": name,
                            "path": rel_path,
                            "size": metadata.len(),
                            "modified": metadata
                                .modified()
                                .ok()
                                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                .map(|d| d.as_secs()),
                            "content_type": content_type,
                            "content": base64_content,
                            "encoding": "base64"
                        });
                        axum::Json(response).into_response()
                    }
                    Err(_) => ApiError::bad_request(format!("Failed to read file: {}", err))
                        .into_response(),
                }
            }
        }
    } else {
        // Return metadata only for binary files
        let response = FileMetadataResponse {
            name,
            path: rel_path,
            size: metadata.len(),
            modified: metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs()),
            content_type,
        };
        axum::Json(response).into_response()
    }
}

/// Normalize and validate file path to prevent directory traversal
fn normalize_path(base_dir: &std::path::Path, file_path: &str) -> Option<std::path::PathBuf> {
    let target = base_dir.join(file_path);
    let canonical_base = base_dir.canonicalize().ok()?;
    let canonical_target = target.canonicalize().ok()?;

    // Ensure target is within base directory
    if canonical_target.starts_with(&canonical_base) {
        Some(canonical_target)
    } else {
        None
    }
}

/// Detect content type based on file extension and content
async fn detect_content_type(file_name: &str, _file_path: &std::path::Path) -> String {
    // Get extension
    let ext = file_name
        .rfind('.')
        .map(|i| &file_name[i + 1..])
        .unwrap_or("")
        .to_lowercase();

    // Map extension to MIME type
    let mime_type = match ext.as_str() {
        // Text files
        "txt" => "text/plain",
        "md" | "markdown" => "text/markdown",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" | "mjs" => "application/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        "yaml" | "yml" => "application/yaml",
        "toml" => "application/toml",
        "rs" => "application/x-rust",
        "py" => "application/x-python-code",
        "sh" => "application/x-sh",
        "bat" => "application/x-bat",
        "ps1" => "application/x-powershell",
        "sql" => "application/sql",
        "log" => "text/plain",
        "ini" => "text/plain",
        "conf" | "config" => "text/plain",
        // Images
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        // Documents
        "pdf" => "application/pdf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        // Archives
        "zip" => "application/zip",
        "tar" => "application/x-tar",
        "gz" | "gzip" => "application/gzip",
        "bz2" => "application/x-bzip2",
        "7z" => "application/x-7z-compressed",
        "rar" => "application/vnd.rar",
        // Audio/Video
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        "avi" => "video/x-msvideo",
        "mov" => "video/quicktime",
        "wav" => "audio/wav",
        // Other
        _ => "application/octet-stream",
    };

    mime_type.to_string()
}

// Base64 encoding utility
mod base64 {
    pub fn encode(bytes: &[u8]) -> String {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

        let mut result = String::with_capacity(bytes.len().saturating_mul(4).div_ceil(3));
        let mut i = 0;

        while i + 3 <= bytes.len() {
            let b0 = bytes[i] as u32;
            let b1 = bytes[i + 1] as u32;
            let b2 = bytes[i + 2] as u32;
            let n = (b0 << 16) | (b1 << 8) | b2;

            result.push(ALPHABET[(n >> 18) as usize] as char);
            result.push(ALPHABET[((n >> 12) & 63) as usize] as char);
            result.push(ALPHABET[((n >> 6) & 63) as usize] as char);
            result.push(ALPHABET[(n & 63) as usize] as char);

            i += 3;
        }

        if i + 2 == bytes.len() {
            let b0 = bytes[i] as u32;
            let b1 = bytes[i + 1] as u32;
            let n = (b0 << 16) | (b1 << 8);

            result.push(ALPHABET[(n >> 18) as usize] as char);
            result.push(ALPHABET[((n >> 12) & 63) as usize] as char);
            result.push(ALPHABET[((n >> 6) & 63) as usize] as char);
            result.push('=');
        } else if i + 1 == bytes.len() {
            let b0 = bytes[i] as u32;
            let n = b0 << 16;

            result.push(ALPHABET[(n >> 18) as usize] as char);
            result.push(ALPHABET[((n >> 12) & 63) as usize] as char);
            result.push('=');
            result.push('=');
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_detect_content_type() {
        // Test with a temporary path
        let temp_dir = std::env::temp_dir();
        
        assert_eq!(
            detect_content_type("test.txt", &temp_dir).await,
            "text/plain"
        );
        assert_eq!(
            detect_content_type("script.py", &temp_dir).await,
            "application/x-python-code"
        );
        assert_eq!(
            detect_content_type("image.png", &temp_dir).await,
            "image/png"
        );
        assert_eq!(
            detect_content_type("doc.pdf", &temp_dir).await,
            "application/pdf"
        );
    }

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64::encode(b"Hello"), "SGVsbG8=");
        assert_eq!(base64::encode(b"Hello, World!"), "SGVsbG8sIFdvcmxkIQ==");
        assert_eq!(base64::encode(b""), "");
        assert_eq!(base64::encode(b"A"), "QQ==");
        assert_eq!(base64::encode(b"AB"), "QUI=");
        assert_eq!(base64::encode(b"ABC"), "QUJD");
    }

    #[tokio::test]
    async fn test_normalize_path() {
        // Create a temporary directory for testing
        let temp_dir = tokio::task::spawn_blocking(|| std::env::temp_dir().join("babata_test_normalize_path"))
            .await
            .unwrap();
        
        // Clean up and create directory
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();
        tokio::fs::write(temp_dir.join("task.md"), "test content").await.unwrap();
        tokio::fs::create_dir_all(temp_dir.join("subdir")).await.unwrap();

        // Valid path within directory
        let result = normalize_path(&temp_dir, "task.md");
        assert!(result.is_some());
        let result_path = result.unwrap();
        assert!(result_path.to_string_lossy().contains("task.md"));

        // Path traversal attack should be blocked
        assert!(normalize_path(&temp_dir, "../../../etc/passwd").is_none());
        assert!(normalize_path(&temp_dir, "..\\..\\..\\etc\\passwd").is_none());
        assert!(normalize_path(&temp_dir, "../task.md").is_none());

        // Clean up
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }
}
