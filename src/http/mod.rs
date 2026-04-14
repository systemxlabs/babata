mod agents;
mod channels;
mod collaborate_task;
mod control_task;
mod count_tasks;
mod create_task;
mod delete_task;
mod file_browser;
mod get_task;
mod get_task_file;
mod get_task_logs;
mod get_task_messages;
mod get_task_tree;
mod list_root_tasks;
mod list_task_files;
mod providers;
mod skills;

mod steer_task;

use std::{env, sync::Arc};

use axum::{
    Json, Router,
    body::Body,
    extract::Request,
    http::{HeaderMap, Method, StatusCode, Uri, Version, header},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use serde_json::json;
use tower_http::services::{ServeDir, ServeFile};
use uuid::Uuid;

use crate::{BabataResult, error::BabataError, task::TaskManager};

pub(crate) use collaborate_task::CollaborateTaskRequest;
pub(crate) use control_task::{ControlTaskRequest, TaskAction};
pub(crate) use create_task::CreateTaskResponse;
pub(crate) use steer_task::SteerTaskRequest;

pub const BABATA_SERVER_PORT_ENV: &str = "BABATA_SERVER_PORT";
pub const DEFAULT_HTTP_HOST: &str = "127.0.0.1";
pub const DEFAULT_HTTP_PORT: u16 = 18800;

#[derive(Clone)]
pub(crate) struct HttpApp {
    pub(crate) task_manager: Arc<TaskManager>,
}

impl HttpApp {
    pub fn new(task_manager: Arc<TaskManager>) -> Self {
        Self { task_manager }
    }

    pub async fn serve(&self) -> BabataResult<()> {
        let http_addr = http_addr()?;
        let listener = tokio::net::TcpListener::bind(&http_addr).await?;

        log::info!("HTTP server listening on {}", http_addr);

        let app = router(self.task_manager.clone());
        axum::serve(listener, app).await.map_err(|err| {
            BabataError::internal(format!("HTTP server stopped unexpectedly: {err}"))
        })
    }
}

fn router(task_manager: Arc<TaskManager>) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/agents", get(agents::list).post(agents::create))
        .route("/api/channels", get(channels::list).post(channels::create))
        .route(
            "/api/channels/{name}",
            put(channels::update).delete(channels::delete),
        )
        .route(
            "/api/agents/{name}",
            get(agents::get).put(agents::update).delete(agents::delete),
        )
        .route("/api/agents/{name}/files", get(agents::list_files))
        .route("/api/agents/{name}/files/{*path}", get(agents::get_file))
        .route(
            "/api/providers",
            get(providers::list).post(providers::create),
        )
        .route(
            "/api/providers/{name}",
            put(providers::update).delete(providers::delete),
        )
        .route("/api/skills", get(skills::list))
        .route("/api/skills/{name}", delete(skills::delete))
        .route("/api/skills/{name}/files", get(skills::list_files))
        .route("/api/skills/{name}/files/{*path}", get(skills::get_file))
        .route("/api/tasks/count", get(count_tasks::handle))
        .route(
            "/api/tasks",
            get(list_root_tasks::handle).post(create_task::handle),
        )
        .route(
            "/api/tasks/{task_id}",
            get(get_task::handle).delete(delete_task::handle),
        )
        .route("/api/tasks/{task_id}/tree", get(get_task_tree::handle))
        .route("/api/tasks/{task_id}/files", get(list_task_files::handle))
        .route(
            "/api/tasks/{task_id}/files/{*path}",
            get(get_task_file::handle),
        )
        .route("/api/tasks/{task_id}/logs", get(get_task_logs::handle))
        .route(
            "/api/tasks/{task_id}/messages",
            get(get_task_messages::handle),
        )
        .route(
            "/api/tasks/{task_id}/collaborate",
            get(collaborate_task::get).post(collaborate_task::create),
        )
        .route("/api/tasks/{task_id}/control", post(control_task::handle))
        .route("/api/tasks/{task_id}/steer", post(steer_task::handle))
        .fallback(serve_web_ui)
        .with_state(HttpApp { task_manager })
}

async fn health() -> impl IntoResponse {
    Json(json!( { "status": "ok" }))
}

async fn serve_web_ui(req: Request) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let version = req.version();
    let headers = req.headers().clone();
    let serve_index = should_serve_spa_index(&method, &uri, &headers);

    let mut static_service = ServeDir::new("web/dist");
    let static_response = static_service
        .try_call(build_static_request(
            method.clone(),
            uri.clone(),
            version,
            headers.clone(),
        ))
        .await
        .expect("serving static files should not fail");

    if static_response.status() != StatusCode::NOT_FOUND || !serve_index {
        return static_response.into_response();
    }

    let mut index_service = ServeFile::new("web/dist/index.html");
    index_service
        .try_call(build_static_request(method, uri, version, headers))
        .await
        .expect("serving index.html should not fail")
        .into_response()
}

fn build_static_request(method: Method, uri: Uri, version: Version, headers: HeaderMap) -> Request {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .version(version)
        .body(Body::empty())
        .expect("static asset request should be valid");
    *request.headers_mut() = headers;
    request
}

fn should_serve_spa_index(method: &Method, uri: &Uri, headers: &HeaderMap) -> bool {
    matches!(method, &Method::GET | &Method::HEAD)
        && !path_has_extension(uri.path())
        && request_accepts_html(headers)
}

fn path_has_extension(path: &str) -> bool {
    path.rsplit('/')
        .next()
        .is_some_and(|segment| segment.contains('.'))
}

fn request_accepts_html(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.contains("text/html"))
}

pub(crate) fn parse_task_id(task_id: &str) -> BabataResult<Uuid> {
    Uuid::parse_str(task_id).map_err(|err| {
        BabataError::invalid_input(format!("Invalid task id '{}': {}", task_id, err))
    })
}

pub(crate) fn ensure_task_exists(task_manager: &TaskManager, task_id: Uuid) -> BabataResult<()> {
    if !task_manager.task_exists(task_id)? {
        return Err(BabataError::not_found(format!(
            "Task '{task_id}' not found",
        )));
    }
    Ok(())
}

pub(crate) fn http_port() -> BabataResult<u16> {
    match env::var(BABATA_SERVER_PORT_ENV) {
        Ok(raw) => raw.parse::<u16>().map_err(|err| {
            BabataError::config(format!(
                "Invalid {BABATA_SERVER_PORT_ENV} value '{}': {}",
                raw, err
            ))
        }),
        Err(env::VarError::NotPresent) => Ok(DEFAULT_HTTP_PORT),
        Err(err) => Err(BabataError::config(format!(
            "Failed to read {BABATA_SERVER_PORT_ENV}: {err}"
        ))),
    }
}

pub(crate) fn http_addr() -> BabataResult<String> {
    Ok(format!("{DEFAULT_HTTP_HOST}:{}", http_port()?))
}

pub(crate) fn http_base_url() -> BabataResult<String> {
    Ok(format!("http://{}", http_addr()?))
}

#[cfg(test)]
mod tests {
    use super::{path_has_extension, request_accepts_html, should_serve_spa_index};
    use axum::http::{HeaderMap, HeaderValue, Method, Uri, header};

    #[test]
    fn spa_fallback_matches_html_navigation_routes() {
        let mut headers = HeaderMap::new();
        headers.insert(header::ACCEPT, HeaderValue::from_static("text/html"));

        assert!(should_serve_spa_index(
            &Method::GET,
            &Uri::from_static("/tasks/123"),
            &headers
        ));
    }

    #[test]
    fn spa_fallback_skips_static_assets() {
        let mut headers = HeaderMap::new();
        headers.insert(header::ACCEPT, HeaderValue::from_static("text/html"));

        assert!(!should_serve_spa_index(
            &Method::GET,
            &Uri::from_static("/assets/index.js"),
            &headers
        ));
        assert!(path_has_extension("/favicon.svg"));
    }

    #[test]
    fn html_accept_detection_requires_text_html() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            HeaderValue::from_static("text/html,application/xhtml+xml"),
        );
        assert!(request_accepts_html(&headers));

        let mut asset_headers = HeaderMap::new();
        asset_headers.insert(
            header::ACCEPT,
            HeaderValue::from_static("text/css,*/*;q=0.1"),
        );
        assert!(!request_accepts_html(&asset_headers));
    }
}
