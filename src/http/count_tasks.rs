use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};

use crate::{BabataResult, error::BabataError, task::TaskStatus};

use super::HttpApp;

pub(super) async fn handle(
    State(state): State<HttpApp>,
    Query(query): Query<CountTasksQuery>,
) -> BabataResult<Json<CountTasksResponse>> {
    let status = match query.status {
        Some(status) => match status.parse::<TaskStatus>() {
            Ok(status) => Some(status),
            Err(err) => return Err(BabataError::invalid_input(err)),
        },
        None => None,
    };

    let count = state.task_manager.count_tasks(status)?;
    Ok(Json(CountTasksResponse { count }))
}

#[derive(Debug, Deserialize)]
pub(super) struct CountTasksQuery {
    #[serde(default)]
    status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct CountTasksResponse {
    pub(crate) count: usize,
}
