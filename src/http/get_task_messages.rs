use std::str::FromStr;

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;

use crate::{BabataResult, error::BabataError, memory::MessageRecord};

use super::{HttpApp, ensure_task_exists, parse_task_id};

const MAX_LIMIT: usize = 1000;

#[derive(Debug, Deserialize)]
pub(crate) struct MessageQueryParams {
    limit: usize,
    #[serde(default)]
    offset: usize,
    message_type: Option<String>,
}

pub(super) async fn handle(
    State(state): State<HttpApp>,
    Path(task_id): Path<String>,
    Query(params): Query<MessageQueryParams>,
) -> BabataResult<Json<Vec<MessageRecord>>> {
    let task_id = parse_task_id(&task_id)?;
    ensure_task_exists(&state.task_manager, task_id)?;

    if params.limit == 0 {
        return Err(BabataError::invalid_input("limit must be greater than 0"));
    }
    if params.limit > MAX_LIMIT {
        return Err(BabataError::invalid_input(format!(
            "limit exceeds maximum value of {}",
            MAX_LIMIT
        )));
    }

    let message_type = match params.message_type {
        Some(ref s) => Some(
            crate::memory::MessageType::from_str(s)
                .map_err(|_| BabataError::invalid_input(format!("Invalid message_type '{}'", s)))?,
        ),
        None => None,
    };

    let messages =
        state
            .task_manager
            .get_task_messages(task_id, params.offset, params.limit, message_type)?;
    Ok(Json(messages))
}
