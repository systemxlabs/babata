use axum::extract::{Path, State};

use crate::BabataResult;

use super::{HttpApp, parse_task_id};

pub(super) async fn handle(
    State(state): State<HttpApp>,
    Path(task_id): Path<String>,
) -> BabataResult<()> {
    let task_id = parse_task_id(&task_id)?;

    state.task_manager.delete_task(task_id)?;
    Ok(())
}
