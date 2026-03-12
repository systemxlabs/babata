use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use chrono::Utc;
use log::info;
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::{
    BabataResult,
    error::BabataError,
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
        info!("Creating task {} with request: {:?}", task_id, request);

        let root_task_id = if let Some(parent_task_id) = request.parent_task_id {
            let task_record = self.store.get_task(parent_task_id)?;
            task_record.root_task_id
        } else {
            task_id
        };

        self.store.insert_task(TaskRecord {
            task_id,
            prompt: request.prompt.clone(),
            agent: request.agent.clone(),
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

    pub fn pause_task(&self, task_id: Uuid) -> BabataResult<()> {
        info!("Pausing task {}", task_id);
        let task = self.store.get_task(task_id)?;
        if task.status != TaskStatus::Running {
            return Err(BabataError::config(format!(
                "Task '{}' cannot be paused from status '{}'",
                task_id, task.status
            )));
        }

        if let Some(running_task) = self.running_tasks.lock().unwrap().remove(&task_id) {
            running_task.handle.abort();
        }

        self.store.update_task_status(task_id, TaskStatus::Paused)?;
        Ok(())
    }

    pub fn resume_task(&self, task_id: Uuid) -> BabataResult<()> {
        info!("Resuming task {}", task_id);
        let task = self.store.get_task(task_id)?;
        if task.status != TaskStatus::Paused {
            return Err(BabataError::config(format!(
                "Task '{}' cannot be resumed from status '{}'",
                task_id, task.status
            )));
        }

        let request = TaskRequest {
            prompt: task.prompt,
            parent_task_id: task.parent_task_id,
            agent: task.agent,
        };
        let running_task = self.launcher.launch(task_id, &request)?;
        {
            let mut guard = self.running_tasks.lock().unwrap();
            guard.insert(task_id, running_task);
        }

        self.store
            .update_task_status(task_id, TaskStatus::Running)?;
        Ok(())
    }

    pub fn cancel_task(&self, task_id: Uuid) -> BabataResult<()> {
        info!("Cancelling task {}", task_id);
        let task = self.store.get_task(task_id)?;
        if matches!(task.status, TaskStatus::Done | TaskStatus::Canceled) {
            return Err(BabataError::config(format!(
                "Task '{}' cannot be canceled from status '{}'",
                task_id, task.status
            )));
        }

        if let Some(running_task) = self.running_tasks.lock().unwrap().remove(&task_id) {
            running_task.handle.abort();
        }

        self.store
            .update_task_status(task_id, TaskStatus::Canceled)?;
        Ok(())
    }

    pub fn list_tasks(
        &self,
        status: Option<TaskStatus>,
        limit: Option<usize>,
    ) -> BabataResult<Vec<TaskRecord>> {
        self.store.list_tasks(status, limit)
    }

    pub fn get_task(&self, task_id: Uuid) -> BabataResult<TaskRecord> {
        self.store.get_task(task_id)
    }
}

#[derive(Debug)]
pub struct RunningTask {
    pub task_id: Uuid,
    pub handle: JoinHandle<()>,
}
