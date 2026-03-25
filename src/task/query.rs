use uuid::Uuid;

use crate::task::TaskStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskListQuery {
    pub status: Option<TaskStatus>,
    pub root_only: bool,
    pub root_task_id: Option<Uuid>,
    pub limit: usize,
    pub offset: Option<usize>,
}

impl Default for TaskListQuery {
    fn default() -> Self {
        Self {
            status: None,
            root_only: false,
            root_task_id: None,
            limit: 100,
            offset: None,
        }
    }
}
