use axum::{
    extract::{Path, Request, State},
    response::{IntoResponse, Response},
};
use tower_http::services::ServeDir;

use crate::{BabataResult, error::BabataError, utils::task_dir};

use super::{HttpApp, ensure_task_exists, file_browser::build_file_request, parse_task_id};

/// Handle GET /api/tasks/{task_id}/files/{*path}
pub(super) async fn handle(
    State(state): State<HttpApp>,
    Path((task_id, file_path)): Path<(String, String)>,
    request: Request,
) -> Response {
    match handle_inner(&state, &task_id, &file_path, request).await {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

async fn handle_inner(
    state: &HttpApp,
    task_id: &str,
    file_path: &str,
    request: Request,
) -> BabataResult<Response> {
    let task_id = parse_task_id(task_id)?;
    ensure_task_exists(&state.task_manager, task_id)?;

    let task_dir = task_dir(task_id)?;
    let forwarded_request = build_file_request(request, file_path)?;

    let mut service = ServeDir::new(task_dir).append_index_html_on_directories(false);
    service
        .try_call(forwarded_request)
        .await
        .map(IntoResponse::into_response)
        .map_err(|err| BabataError::internal(format!("Failed to serve task file: {err}")))
}
