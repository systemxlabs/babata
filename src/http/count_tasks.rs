use axum::{
    Json,
    extract::{Query, State},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::task::TaskStatus;

use super::{ApiError, HttpApp};

pub(super) async fn handle(
    State(state): State<HttpApp>,
    Query(query): Query<CountTasksQuery>,
) -> Response {
    let status = match query.status {
        Some(status) => match status.parse::<TaskStatus>() {
            Ok(status) => Some(status),
            Err(err) => return ApiError::bad_request(err).into_response(),
        },
        None => None,
    };

    match state.task_manager.count_tasks(status) {
        Ok(count) => Json(CountTasksResponse { count }).into_response(),
        Err(err) => ApiError::from_babata_error(err).into_response(),
    }
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
