mod collaborate_task;
mod control_task;
mod count_tasks;
mod create_agent;
mod create_task;
mod delete_agent;
mod delete_skill;
mod delete_task;
mod get_agent;
mod get_task;
mod get_task_tree;
mod get_task_file;
mod get_task_logs;
mod list_agents;
mod list_root_tasks;
mod list_skills;
mod list_task_files;

mod steer_task;
mod update_agent;

use std::{env, sync::Arc};

use axum::{
    Json, Router,
    response::IntoResponse,
    routing::{delete, get, post},
};
use serde_json::json;
use tower_http::services::ServeDir;
use uuid::Uuid;

use crate::{BabataResult, error::BabataError, task::TaskManager};

pub(crate) use collaborate_task::CollaborateTaskRequest;
pub(crate) use control_task::{ControlTaskRequest, TaskAction};
pub(crate) use count_tasks::CountTasksResponse;
pub(crate) use create_task::CreateTaskResponse;
pub(crate) use get_task::TaskResponse;
pub(crate) use list_root_tasks::{ListRootTasksResponse, RootTaskResponse};
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
        .route(
            "/api/agents",
            get(list_agents::handle).post(create_agent::handle),
        )
        .route(
            "/api/agents/{name}",
            get(get_agent::handle)
                .put(update_agent::handle)
                .delete(delete_agent::handle),
        )
        .route("/api/skills", get(list_skills::handle))
        .route("/api/skills/{name}", delete(delete_skill::handle))
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
            "/api/tasks/{task_id}/collaborate",
            get(collaborate_task::get).post(collaborate_task::create),
        )
        .route("/api/tasks/{task_id}/control", post(control_task::handle))
        .route("/api/tasks/{task_id}/steer", post(steer_task::handle))
        .fallback_service(ServeDir::new("web/dist"))
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
