mod assets;
mod control_task;
mod count_tasks;
mod create_task;
mod error;
mod get_task;
mod list_tasks;

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::HeaderMap,
    response::IntoResponse,
    routing::{get, post},
};
use serde_json::json;

use crate::{BabataResult, error::BabataError, task::TaskManager};

pub(crate) use control_task::RelaunchTaskRequest;
pub(crate) use count_tasks::CountTasksResponse;
pub(crate) use error::ApiError;
pub(crate) use get_task::TaskResponse;
pub(crate) use list_tasks::ListTasksResponse;

pub const DEFAULT_HTTP_ADDR: &str = "127.0.0.1:18800";
pub const DEFAULT_HTTP_BASE_URL: &str = "http://127.0.0.1:18800";

#[derive(Clone)]
pub(crate) struct HttpApp {
    pub(crate) task_manager: Arc<TaskManager>,
}

impl HttpApp {
    pub fn new(task_manager: Arc<TaskManager>) -> Self {
        Self { task_manager }
    }

    pub async fn serve(&self) -> BabataResult<()> {
        let listener = tokio::net::TcpListener::bind(DEFAULT_HTTP_ADDR)
            .await
            .map_err(|err| {
                BabataError::internal(format!(
                    "Failed to bind HTTP server on {DEFAULT_HTTP_ADDR}: {err}"
                ))
            })?;

        log::info!("HTTP server listening on {}", DEFAULT_HTTP_ADDR);

        let app = router(self.task_manager.clone());
        axum::serve(listener, app).await.map_err(|err| {
            BabataError::internal(format!("HTTP server stopped unexpectedly: {err}"))
        })
    }
}

fn router(task_manager: Arc<TaskManager>) -> Router {
    Router::new()
        .route("/", get(assets::spa_shell))
        .route("/health", get(health))
        .route("/tasks/count", get(count_tasks::handle))
        .route("/tasks", get(list_tasks_or_shell).post(create_task::handle))
        .route("/tasks/{task_id}", get(get_task_or_shell))
        .route("/tasks/{task_id}/pause", post(control_task::pause))
        .route("/tasks/{task_id}/resume", post(control_task::resume))
        .route("/tasks/{task_id}/cancel", post(control_task::cancel))
        .route("/tasks/{task_id}/relaunch", post(control_task::relaunch))
        .route("/create", get(assets::spa_shell))
        .route("/system", get(assets::spa_shell))
        .route("/assets/{*path}", get(assets::static_asset))
        .with_state(HttpApp { task_manager })
}

pub fn router_for_test() -> Router {
    Router::new()
        .route("/", get(assets::spa_shell))
        .route("/create", get(assets::spa_shell))
        .route("/system", get(assets::spa_shell))
        .route("/assets/{*path}", get(assets::static_asset))
}

async fn health() -> impl IntoResponse {
    Json(json!( { "status": "ok" }))
}

async fn list_tasks_or_shell(
    State(state): State<HttpApp>,
    headers: HeaderMap,
    query: Query<list_tasks::ListTasksQuery>,
) -> impl IntoResponse {
    if assets::accepts_html(&headers) {
        return assets::shell_response();
    }

    list_tasks::handle(State(state), query).await
}

async fn get_task_or_shell(
    State(state): State<HttpApp>,
    headers: HeaderMap,
    path: Path<String>,
) -> impl IntoResponse {
    if assets::accepts_html(&headers) {
        return assets::shell_response();
    }

    get_task::handle(State(state), path).await
}
