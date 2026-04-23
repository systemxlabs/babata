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
    #[serde(default)]
    message_type: Option<crate::memory::MessageType>,
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

    let message_type = params.message_type;

    let messages =
        state
            .task_manager
            .get_task_messages(task_id, params.offset, params.limit, message_type)?;
    Ok(Json(messages))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_query_params_deserialization() {
        let json_valid = r#"{"limit": 10, "message_type": "user_prompt"}"#;
        let res1: Result<MessageQueryParams, _> = serde_json::from_str(json_valid);
        assert!(res1.is_ok());
        assert_eq!(
            res1.unwrap().message_type,
            Some(crate::memory::MessageType::UserPrompt)
        );

        let json_invalid = r#"{"limit": 10, "message_type": "unknown"}"#;
        let res2: Result<MessageQueryParams, _> = serde_json::from_str(json_invalid);
        assert!(res2.is_err());

        let json_none = r#"{"limit": 10}"#;
        let res3: Result<MessageQueryParams, _> = serde_json::from_str(json_none);
        assert!(res3.is_ok());
        assert_eq!(res3.unwrap().message_type, None);
    }
}
