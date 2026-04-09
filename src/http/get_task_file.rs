use axum::{
    body::Body,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};

use super::{ApiError, HttpApp, ensure_task_exists, parse_task_id};

/// Handle GET /api/tasks/{task_id}/files/{*path}
/// Returns raw file content for text files, raw binary bytes for binary files
pub(super) async fn handle(
    State(state): State<HttpApp>,
    Path((task_id, file_path)): Path<(String, String)>,
) -> Response {
    match handle_inner(&state, &task_id, &file_path).await {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

async fn handle_inner(
    state: &HttpApp,
    task_id: &str,
    file_path: &str,
) -> Result<Response, ApiError> {
    let task_id = parse_task_id(task_id)?;
    ensure_task_exists(&state.task_manager, task_id)?;

    let task_dir = crate::utils::babata_dir()
        .map_err(ApiError::from)?
        .join("tasks")
        .join(task_id.to_string());

    let file_path = file_path.replace('/', std::path::MAIN_SEPARATOR_STR);
    let target_path = normalize_path(&task_dir, &file_path)
        .ok_or_else(|| ApiError::bad_request("Invalid file path: directory traversal detected"))?;

    let metadata = tokio::fs::metadata(&target_path)
        .await
        .map_err(|err| ApiError::bad_request(format!("File not found: {}", err)))?;
    if !metadata.is_file() {
        return Err(ApiError::bad_request(format!(
            "'{}' is not a file (might be a directory)",
            file_path
        )));
    }

    let name = target_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| file_path.clone());
    let content_type = detect_content_type(&name, &target_path).await;

    let body = if is_text_content_type(&content_type) {
        match tokio::fs::read_to_string(&target_path).await {
            Ok(content) => Body::from(content),
            Err(text_err) => {
                let bytes = tokio::fs::read(&target_path).await.map_err(|_| {
                    ApiError::bad_request(format!("Failed to read file: {}", text_err))
                })?;
                Body::from(bytes)
            }
        }
    } else {
        let bytes = tokio::fs::read(&target_path)
            .await
            .map_err(|err| ApiError::bad_request(format!("Failed to read file: {}", err)))?;
        Body::from(bytes)
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .body(body)
        .map_err(|_| {
            ApiError::from(crate::error::BabataError::internal(
                "Failed to build response",
            ))
        })
}

fn is_text_content_type(content_type: &str) -> bool {
    content_type.starts_with("text/")
        || content_type == "application/json"
        || content_type == "application/javascript"
        || content_type == "application/xml"
        || content_type == "application/yaml"
        || content_type == "application/toml"
        || content_type == "application/x-sh"
        || content_type == "application/x-httpd-php"
        || content_type == "application/x-python-code"
        || content_type == "application/x-rust"
        || content_type.starts_with("application/x-")
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

    #[tokio::test]
    async fn test_normalize_path() {
        // Create a temporary directory for testing
        let temp_dir =
            tokio::task::spawn_blocking(|| std::env::temp_dir().join("babata_test_normalize_path"))
                .await
                .unwrap();

        // Clean up and create directory
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();
        tokio::fs::write(temp_dir.join("task.md"), "test content")
            .await
            .unwrap();
        tokio::fs::create_dir_all(temp_dir.join("subdir"))
            .await
            .unwrap();

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
