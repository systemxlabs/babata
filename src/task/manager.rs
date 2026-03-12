use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use chrono::Utc;
use uuid::Uuid;

use crate::{
    BabataResult,
    task::{TaskRecord, TaskRequest, TaskStatus, TaskStore, launcher::TaskLauncher},
};

#[derive(Debug)]
pub struct TaskManager {
    store: TaskStore,
    launcher: TaskLauncher,
    running_tasks: Arc<Mutex<HashMap<Uuid, RunningTask>>>,
}

impl TaskManager {
    pub fn new(launcher: TaskLauncher) -> BabataResult<Self> {
        Ok(Self {
            store: TaskStore::new()?,
            launcher,
            running_tasks: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn create_task(&self, request: TaskRequest) -> BabataResult<Uuid> {
        let task_id = Uuid::new_v4();

        let root_task_id = if let Some(parent_task_id) = request.parent_task_id {
            let task_record = self.store.get_task(parent_task_id)?;
            task_record.root_task_id
        } else {
            task_id
        };

        self.store.insert_task(TaskRecord {
            task_id,
            status: TaskStatus::Running,
            parent_task_id: request.parent_task_id,
            root_task_id,
            created_at: Utc::now().timestamp_millis(),
        })?;

        let running_task = self.launcher.launch(task_id, &request)?;
        {
            let mut guard = self.running_tasks.lock().unwrap();
            guard.insert(task_id, running_task);
        }

        Ok(task_id)
    }
}

#[derive(Debug)]
pub struct RunningTask {
    pub task_id: Uuid,
    pub handle: tokio::task::JoinHandle<()>,
}
