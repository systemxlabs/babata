use uuid::Uuid;

use crate::{BabataResult, task::TaskStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRecord {
    pub task_id: Uuid,
    pub status: TaskStatus,
    pub parent_task_id: Option<Uuid>,
    pub root_task_id: Uuid,
    pub created_at: i64,
}

#[derive(Debug)]
pub struct TaskStore {}

impl TaskStore {
    pub fn new() -> BabataResult<Self> {
        unimplemented!()
    }

    pub fn insert_task(&self, record: TaskRecord) -> BabataResult<()> {
        unimplemented!()
    }

    pub fn update_task_status(&self, task_id: Uuid, status: TaskStatus) -> BabataResult<()> {
        unimplemented!()
    }

    pub fn get_task(&self, task_id: Uuid) -> BabataResult<TaskRecord> {
        unimplemented!()
    }
}
