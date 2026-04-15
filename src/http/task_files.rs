use axum::{
    Json,
    extract::{Path, Request, State},
    response::{IntoResponse, Response},
};
use tower_http::services::ServeDir;

use crate::{BabataResult, error::BabataError, utils::task_dir};

use super::{
    HttpApp, ensure_task_exists,
    file_browser::{BrowsedPath, FileEntry, browse_path, build_file_request},
    parse_task_id,
};

/// Handle GET /api/tasks/{task_id}/files
pub(super) async fn list(
    State(state): State<HttpApp>,
    Path(task_id): Path<String>,
    request: Request,
) -> Response {
    match handle_inner(&state, &task_id, None, request).await {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

/// Handle GET /api/tasks/{task_id}/files/{*path}
pub(super) async fn get(
    State(state): State<HttpApp>,
    Path((task_id, file_path)): Path<(String, String)>,
    request: Request,
) -> Response {
    match handle_inner(&state, &task_id, Some(&file_path), request).await {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

async fn handle_inner(
    state: &HttpApp,
    task_id: &str,
    file_path: Option<&str>,
    request: Request,
) -> BabataResult<Response> {
    let task_id = parse_task_id(task_id)?;
    ensure_task_exists(&state.task_manager, task_id)?;

    let task_dir = task_dir(task_id)?;

    if !task_dir.exists() {
        return Ok(Json(Vec::<FileEntry>::new()).into_response());
    }

    match browse_path(&task_dir, file_path).await? {
        BrowsedPath::Directory(entries) => Ok(Json(entries).into_response()),
        BrowsedPath::File(sanitized_path) => {
            let forwarded_request = build_file_request(request, &sanitized_path)?;
            let mut service = ServeDir::new(task_dir).append_index_html_on_directories(false);
            service
                .try_call(forwarded_request)
                .await
                .map(IntoResponse::into_response)
                .map_err(|err| BabataError::internal(format!("Failed to serve task file: {err}")))
        }
    }
}
