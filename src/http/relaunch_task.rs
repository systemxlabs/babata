use axum::{
    Json,
    extract::{Path, State},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::BabataResult;

use super::{HttpApp, parse_task_id, require_non_empty};

pub(super) async fn handle(
    State(state): State<HttpApp>,
    Path(task_id): Path<String>,
    Json(request): Json<RelaunchTaskRequest>,
) -> BabataResult<()> {
    let task_id = parse_task_id(&task_id)?;
    require_non_empty(&request.reason, "reason")?;

    state
        .task_manager
        .relaunch_task(task_id, request.reason.trim())?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RelaunchTaskRequest {
    pub(crate) reason: String,
}

#[cfg(test)]
mod tests {
    use super::RelaunchTaskRequest;
    use serde_json::json;

    #[test]
    fn relaunch_task_request_deserializes_reason() {
        let request = serde_json::from_value::<RelaunchTaskRequest>(json!({
            "reason": "resume with new provider config"
        }))
        .expect("deserialize request");

        assert_eq!(request.reason, "resume with new provider config");
    }

    #[test]
    fn relaunch_task_request_rejects_unknown_fields() {
        let error = serde_json::from_value::<RelaunchTaskRequest>(json!({
            "reason": "retry",
            "task_id": "12345678-1234-1234-1234-123456789abc"
        }))
        .expect_err("unknown field should fail");

        assert!(error.to_string().contains("unknown field"));
    }
}
