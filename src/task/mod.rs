mod launcher;
mod manager;
mod store;

pub use launcher::*;
pub use manager::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
pub use store::*;

use crate::{error::BabataError, message::Content};
use uuid::Uuid;

/// Steer message sent to a running task to influence its behavior.
#[derive(Debug, Clone)]
pub struct SteerMessage {
    pub content: Vec<Content>,
}

impl SteerMessage {
    pub fn new(content: Vec<Content>) -> Self {
        Self { content }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub description: String,
    pub prompt: Vec<Content>,
    #[serde(default)]
    pub parent_task_id: Option<Uuid>,
    pub agent: String,
    pub never_ends: bool,
}

#[cfg(test)]
mod tests {
    use super::CreateTaskRequest;
    use crate::message::Content;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn create_task_request_requires_never_ends_when_deserializing() {
        let error = serde_json::from_value::<CreateTaskRequest>(json!({
            "description": "hello",
            "prompt": [{ "type": "text", "text": "hello" }],
            "agent": "babata",
        }))
        .expect_err("missing never_ends should fail");

        assert!(error.to_string().contains("never_ends"));
    }

    #[test]
    fn create_task_request_deserializes_with_explicit_never_ends() {
        let parent_task_id = Uuid::new_v4();
        let request = serde_json::from_value::<CreateTaskRequest>(json!({
            "description": "demo task",
            "prompt": [{ "type": "text", "text": "hello" }],
            "parent_task_id": parent_task_id,
            "agent": "babata",
            "never_ends": true,
        }))
        .expect("request should deserialize");

        assert_eq!(
            request.prompt,
            vec![Content::Text {
                text: "hello".to_string()
            }]
        );
        assert_eq!(request.description, "demo task");
        assert_eq!(request.parent_task_id, Some(parent_task_id));
        assert_eq!(request.agent, "babata");
        assert!(request.never_ends);
    }

    #[test]
    fn create_task_request_requires_agent_when_deserializing() {
        let error = serde_json::from_value::<CreateTaskRequest>(json!({
            "description": "hello",
            "prompt": [{ "type": "text", "text": "hello" }],
            "never_ends": false,
        }))
        .expect_err("missing agent should fail");

        assert!(error.to_string().contains("agent"));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    #[default]
    Running,
    Done,
    Canceled,
    Paused,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            TaskStatus::Running => "running",
            TaskStatus::Done => "done",
            TaskStatus::Canceled => "canceled",
            TaskStatus::Paused => "paused",
        };
        f.write_str(value)
    }
}

impl std::str::FromStr for TaskStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "running" => Ok(TaskStatus::Running),
            "done" => Ok(TaskStatus::Done),
            "canceled" => Ok(TaskStatus::Canceled),
            "paused" => Ok(TaskStatus::Paused),
            _ => Err(format!("Unknown task status '{}'", s)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CollaborationTaskState {
    NonExisting,
    Running,
    Failed { reason: String },
    Succeed { result: Vec<Content> },
}

#[derive(Debug)]
pub enum TaskExitEvent {
    Completed { task_id: Uuid },
    Failed { task_id: Uuid, error: BabataError },
}
