mod collaborate_task;
mod control_task;
mod count_tasks;
mod create_task;
mod delete_task;
mod get_task;
mod get_task_file;
mod get_task_logs;
mod list_task_files;
mod list_tasks;
mod steer_task;

use std::sync::Arc;

use axum::{
    Json, Router,
    response::IntoResponse,
    routing::{get, post},
};
use serde_json::json;
use uuid::Uuid;

use crate::{BabataResult, error::BabataError, task::TaskManager};

pub(crate) use collaborate_task::CollaborateTaskRequest;
pub(crate) use control_task::{ControlTaskRequest, TaskAction};
pub(crate) use count_tasks::CountTasksResponse;
pub(crate) use get_task::TaskResponse;
pub(crate) use steer_task::SteerTaskRequest;

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
        let listener = tokio::net::TcpListener::bind(DEFAULT_HTTP_ADDR).await?;

        log::info!("HTTP server listening on {}", DEFAULT_HTTP_ADDR);

        let app = router(self.task_manager.clone());
        axum::serve(listener, app).await.map_err(|err| {
            BabataError::internal(format!("HTTP server stopped unexpectedly: {err}"))
        })
    }
}

fn router(task_manager: Arc<TaskManager>) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/tasks/count", get(count_tasks::handle))
        .route(
            "/api/tasks",
            get(list_tasks::handle).post(create_task::handle),
        )
        .route(
            "/api/tasks/{task_id}",
            get(get_task::handle).delete(delete_task::handle),
        )
        .route("/api/tasks/{task_id}/files", get(list_task_files::handle))
        .route(
            "/api/tasks/{task_id}/files/{*path}",
            get(get_task_file::handle),
        )
        .route("/api/tasks/{task_id}/logs", get(get_task_logs::handle))
        .route(
            "/api/tasks/{task_id}/collaborate",
            get(collaborate_task::get).post(collaborate_task::create),
        )
        .route("/api/tasks/{task_id}/control", post(control_task::handle))
        .route("/api/tasks/{task_id}/steer", post(steer_task::handle))
        .with_state(HttpApp { task_manager })
}

async fn health() -> impl IntoResponse {
    Json(json!( { "status": "ok" }))
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
