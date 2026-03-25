mod assets;
mod control_task;
mod count_tasks;
mod create_task;
mod error;
mod get_overview;
mod get_system;
mod get_task;
mod get_task_content;
mod get_task_logs;
mod get_task_tree;
mod list_task_artifacts;
mod list_tasks;

use std::{collections::HashMap, fs, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, Uri},
    response::IntoResponse,
    routing::{get, post},
};
use serde_json::json;

use crate::{
    BabataResult,
    config::{AgentConfig, CodexAgentConfig, Config},
    error::BabataError,
    task::{TaskLauncher, TaskManager, TaskStore},
};

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
    let api = Router::new()
        .route("/overview", get(get_overview::handle))
        .route("/system", get(get_system::handle))
        .route(
            "/tasks",
            get(list_tasks::handle_api).post(create_task::handle),
        )
        .route("/tasks/{task_id}/content", get(get_task_content::handle))
        .route("/tasks/{task_id}/tree", get(get_task_tree::handle))
        .route(
            "/tasks/{task_id}/artifacts",
            get(list_task_artifacts::handle),
        )
        .route("/tasks/{task_id}/logs", get(get_task_logs::handle))
        .route("/tasks/{task_id}", get(get_task::handle))
        .route("/tasks/{task_id}/pause", post(control_task::pause))
        .route("/tasks/{task_id}/resume", post(control_task::resume))
        .route("/tasks/{task_id}/cancel", post(control_task::cancel))
        .route("/tasks/{task_id}/relaunch", post(control_task::relaunch));

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
        .nest("/api", api)
        .route("/create", get(assets::spa_shell))
        .route("/system", get(assets::spa_shell))
        .route("/assets/{*path}", get(assets::static_asset))
        .with_state(HttpApp { task_manager })
}

pub fn router_for_test() -> Router {
    router(Arc::new(build_test_task_manager()))
}

async fn health() -> impl IntoResponse {
    Json(json!( { "status": "ok" }))
}

async fn list_tasks_or_shell(
    State(state): State<HttpApp>,
    headers: HeaderMap,
    uri: Uri,
) -> impl IntoResponse {
    if assets::prefers_html(&headers) {
        return assets::shell_response();
    }

    let query = match Query::<list_tasks::ListTasksQuery>::try_from_uri(&uri) {
        Ok(query) => query,
        Err(err) => return ApiError::bad_request(err.to_string()).into_response(),
    };

    list_tasks::handle(State(state), query).await
}

async fn get_task_or_shell(
    State(state): State<HttpApp>,
    headers: HeaderMap,
    path: Path<String>,
) -> impl IntoResponse {
    if assets::prefers_html(&headers) {
        return assets::shell_response();
    }

    get_task::handle(State(state), path).await
}

fn build_test_task_manager() -> TaskManager {
    let test_root =
        std::env::temp_dir().join(format!("babata-http-router-test-{}", uuid::Uuid::new_v4()));
    let workspace = test_root.join("workspace");
    fs::create_dir_all(&workspace).expect("create HTTP test workspace");

    let config = Config {
        providers: Vec::new(),
        agents: vec![AgentConfig::Codex(CodexAgentConfig {
            command: test_agent_command(),
            workspace: workspace.display().to_string(),
            model: None,
        })],
        channels: Vec::new(),
        memory: Vec::new(),
    };

    let store = TaskStore::open(test_root.join("task.db")).expect("open HTTP test task store");
    let launcher =
        TaskLauncher::new(&config, HashMap::new()).expect("build HTTP test task launcher");

    TaskManager::new(store, launcher).expect("build HTTP test task manager")
}

fn test_agent_command() -> String {
    if cfg!(windows) {
        "cmd".to_string()
    } else {
        "true".to_string()
    }
}
