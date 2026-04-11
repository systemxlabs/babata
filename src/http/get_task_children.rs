use axum::{
    Json,
    extract::{Path, State},
};
use serde::Serialize;

use crate::BabataResult;

use super::{HttpApp, get_task::TaskResponse, parse_task_id};

pub(super) async fn handle(
    State(state): State<HttpApp>,
    Path(task_id): Path<String>,
) -> BabataResult<Json<TaskChildrenResponse>> {
    let task_id = parse_task_id(&task_id)?;

    let children = state.task_manager.get_task_children(task_id)?;
    
    Ok(Json(TaskChildrenResponse {
        children: children.into_iter().map(TaskResponse::from_record).collect(),
    }))
}

#[derive(Debug, Serialize)]
pub(crate) struct TaskChildrenResponse {
    pub(crate) children: Vec<TaskResponse>,
}
