use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::task::TaskStatus;

use super::{ApiError, HttpApp, get_task::TaskResponse};

pub(super) async fn handle(State(state): State<HttpApp>) -> Response {
    let status_counts = match StatusCounts::load(&state) {
        Ok(counts) => counts,
        Err(err) => return err.into_response(),
    };

    let recent_tasks = match state.task_manager.list_tasks(None, 10, None) {
        Ok(tasks) => tasks.into_iter().map(TaskResponse::from_record).collect(),
        Err(err) => return ApiError::from_babata_error(err).into_response(),
    };

    Json(OverviewResponse {
        status_counts,
        recent_tasks,
    })
    .into_response()
}

#[derive(Debug, Serialize)]
struct OverviewResponse {
    status_counts: StatusCounts,
    recent_tasks: Vec<TaskResponse>,
}

#[derive(Debug, Serialize)]
struct StatusCounts {
    total: usize,
    running: usize,
    paused: usize,
    canceled: usize,
    done: usize,
}

impl StatusCounts {
    fn load(state: &HttpApp) -> Result<Self, ApiError> {
        let total = state
            .task_manager
            .count_tasks(None)
            .map_err(ApiError::from_babata_error)?;
        let running = state
            .task_manager
            .count_tasks(Some(TaskStatus::Running))
            .map_err(ApiError::from_babata_error)?;
        let paused = state
            .task_manager
            .count_tasks(Some(TaskStatus::Paused))
            .map_err(ApiError::from_babata_error)?;
        let canceled = state
            .task_manager
            .count_tasks(Some(TaskStatus::Canceled))
            .map_err(ApiError::from_babata_error)?;
        let done = state
            .task_manager
            .count_tasks(Some(TaskStatus::Done))
            .map_err(ApiError::from_babata_error)?;

        Ok(Self {
            total,
            running,
            paused,
            canceled,
            done,
        })
    }
}

