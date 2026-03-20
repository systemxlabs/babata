mod control_task;
mod count_tasks;
mod create_task;
mod error;
mod get_task;
mod list_tasks;

use std::sync::Arc;

use axum::{
    Json, Router,
    response::IntoResponse,
    routing::{get, post},
};
use serde_json::json;

use crate::{BabataResult, error::BabataError, task::TaskManager};

pub(crate) use control_task::RelaunchTaskRequest;
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
        .route("/health", get(health))
        .route("/tasks/count", get(count_tasks::handle))
        .route("/tasks", get(list_tasks::handle).post(create_task::handle))
        .route("/tasks/{task_id}", get(get_task::handle))
        .route("/tasks/{task_id}/pause", post(control_task::pause))
        .route("/tasks/{task_id}/resume", post(control_task::resume))
        .route("/tasks/{task_id}/cancel", post(control_task::cancel))
        .route("/tasks/{task_id}/relaunch", post(control_task::relaunch))
        .with_state(HttpApp { task_manager })
}

async fn health() -> impl IntoResponse {
    Json(json!( { "status": "ok" }))
}
