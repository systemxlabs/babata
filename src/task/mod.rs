mod launcher;
mod manager;
mod store;

pub use launcher::*;
pub use manager::*;
use serde::{Deserialize, Serialize};
pub use store::*;

use crate::{error::BabataError, message::Content};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct TaskRequest {
    pub prompt: Vec<Content>,
    pub parent_task_id: Option<Uuid>,
    pub agent: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TaskStatus {
    #[default]
    Running,
    Done,
    Canceled,
    Paused,
}

impl TaskStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            TaskStatus::Running => "running",
            TaskStatus::Done => "done",
            TaskStatus::Canceled => "canceled",
            TaskStatus::Paused => "paused",
        }
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
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

#[derive(Debug)]
pub enum TaskExitEvent {
    Completed { task_id: Uuid },
    Failed { task_id: Uuid, error: BabataError },
}
