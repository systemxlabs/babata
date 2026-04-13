use axum::{
    extract::{Path, Request},
    response::{IntoResponse, Response},
};
use tower_http::services::ServeDir;

use crate::{
    BabataResult,
    agent::{agent_dir, agent_exists},
    error::BabataError,
};

use super::file_browser::build_file_request;

/// Handle GET /api/agents/{name}/files/{*path}
pub(super) async fn handle(
    Path((name, file_path)): Path<(String, String)>,
    request: Request,
) -> Response {
    match handle_inner(&name, &file_path, request).await {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

async fn handle_inner(name: &str, file_path: &str, request: Request) -> BabataResult<Response> {
    if !agent_exists(name) {
        return Err(BabataError::not_found(format!(
            "Agent '{}' not found",
            name
        )));
    }

    let agent_dir = agent_dir(name)?;
    let forwarded_request = build_file_request(request, file_path)?;

    let mut service = ServeDir::new(agent_dir).append_index_html_on_directories(false);
    service
        .try_call(forwarded_request)
        .await
        .map(IntoResponse::into_response)
        .map_err(|err| BabataError::internal(format!("Failed to serve agent file: {err}")))
}
