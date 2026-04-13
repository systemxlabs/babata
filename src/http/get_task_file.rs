use axum::{
    body::Body,
    extract::{Path, Request, State},
    http::Uri,
    response::{IntoResponse, Response},
};
use tower_http::services::ServeDir;

use crate::{BabataResult, error::BabataError, utils::task_dir};

use super::{HttpApp, ensure_task_exists, parse_task_id};

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
    let forwarded_request = build_task_file_request(request, file_path)?;

    let mut service = ServeDir::new(task_dir).append_index_html_on_directories(false);
    service
        .try_call(forwarded_request)
        .await
        .map(IntoResponse::into_response)
        .map_err(|err| BabataError::internal(format!("Failed to serve task file: {err}")))
}

fn build_task_file_request(request: Request, file_path: &str) -> BabataResult<Request> {
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
        .map_err(|_| BabataError::internal("Failed to build forwarded task file request"))?;
    *forwarded_request.headers_mut() = headers;

    Ok(forwarded_request)
}

#[cfg(test)]
mod tests {
    use super::build_task_file_request;
    use axum::{
        body::Body,
        extract::Request,
        http::{Method, Version, header},
    };

    #[test]
    fn rewrites_api_request_to_task_relative_uri() {
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/tasks/task-id/files/logs/output.txt")
            .version(Version::HTTP_11)
            .header(header::ACCEPT, "text/plain")
            .body(Body::empty())
            .expect("request");

        let forwarded = build_task_file_request(request, "logs/output.txt").expect("forwarded");

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

        let forwarded = build_task_file_request(request, "logs\\output.txt").expect("forwarded");

        assert_eq!(forwarded.uri().path(), "/logs/output.txt");
    }
}
