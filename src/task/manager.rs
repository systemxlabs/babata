use std::{collections::HashMap, sync::Arc};

use chrono::Utc;
use log::{error, info, warn};
use parking_lot::Mutex;
use tokio::{sync::mpsc, task::JoinHandle};
use uuid::Uuid;

use crate::{
    BabataResult,
    error::BabataError,
    task::{
        TaskExitEvent, TaskRecord, TaskRequest, TaskStatus, TaskStore, launcher::TaskLauncher,
        task_dir,
    },
};

pub struct TaskManager {
    store: TaskStore,
    launcher: TaskLauncher,
    running_tasks: Arc<Mutex<HashMap<Uuid, RunningTask>>>,
    exit_tx: mpsc::Sender<TaskExitEvent>,
    exit_rx: Mutex<Option<mpsc::Receiver<TaskExitEvent>>>,
}

impl TaskManager {
    pub fn new(store: TaskStore, launcher: TaskLauncher) -> BabataResult<Self> {
        let (exit_tx, exit_rx) = mpsc::channel(1024);
        Ok(Self {
            store,
            launcher,
            running_tasks: Arc::new(Mutex::new(HashMap::new())),
            exit_tx,
            exit_rx: Mutex::new(Some(exit_rx)),
        })
    }

    pub fn start(self: &Arc<Self>) -> BabataResult<()> {
        let Some(mut exit_rx) = self.exit_rx.lock().take() else {
            return Err(BabataError::internal(
                "Task manager exit loop has already been started",
            ));
        };

        let task_manager = Arc::clone(self);
        tokio::spawn(async move {
            while let Some(event) = exit_rx.recv().await {
                task_manager.handle_task_exit(event);
            }
        });

        self.recover_running_tasks()?;
        Ok(())
    }

    fn recover_running_tasks(&self) -> BabataResult<()> {
        let tasks = self
            .store
            .list_tasks(Some(TaskStatus::Running), None, None)?;
        if tasks.is_empty() {
            info!("No running tasks to recover on startup");
            return Ok(());
        }

        info!("Recovering {} running task(s) from task store", tasks.len());
        for task in tasks {
            if self.running_tasks.lock().contains_key(&task.task_id) {
                info!(
                    "Skipping recovery for task {} because it is already running",
                    task.task_id
                );
                continue;
            }

            self.ensure_task_dir(task.task_id)?;
            let running_task = self.launcher.launch(&task, self.exit_tx.clone())?;
            self.running_tasks.lock().insert(task.task_id, running_task);
            info!("Recovered running task {}", task.task_id);
        }

        Ok(())
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

        let task_record = TaskRecord {
            task_id,
            prompt: request.prompt.clone(),
            agent: request.agent.clone(),
            status: TaskStatus::Running,
            parent_task_id: request.parent_task_id,
            root_task_id,
            created_at: Utc::now().timestamp_millis(),
        };
        self.store.insert_task(task_record.clone())?;
        self.ensure_task_dir(task_id)?;

        let running_task = self.launcher.launch(&task_record, self.exit_tx.clone())?;
        {
            let mut guard = self.running_tasks.lock();
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

        if let Some(running_task) = self.running_tasks.lock().remove(&task_id) {
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

        let running_task = self.launcher.launch(&task, self.exit_tx.clone())?;
        {
            let mut guard = self.running_tasks.lock();
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

        if let Some(running_task) = self.running_tasks.lock().remove(&task_id) {
            running_task.handle.abort();
        }

        self.store
            .update_task_status(task_id, TaskStatus::Canceled)?;
        self.remove_task_dir(task_id);
        Ok(())
    }

    pub fn list_tasks(
        &self,
        status: Option<TaskStatus>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> BabataResult<Vec<TaskRecord>> {
        self.store.list_tasks(status, limit, offset)
    }

    pub fn get_task(&self, task_id: Uuid) -> BabataResult<TaskRecord> {
        self.store.get_task(task_id)
    }

    fn handle_task_exit(&self, event: TaskExitEvent) {
        match event {
            TaskExitEvent::Completed { task_id } => self.handle_task_completed(task_id),
            TaskExitEvent::Failed { task_id, error } => self.handle_task_failed(task_id, error),
        }
    }

    fn handle_task_completed(&self, task_id: Uuid) {
        self.running_tasks.lock().remove(&task_id);

        let task = match self.store.get_task(task_id) {
            Ok(task) => task,
            Err(err) => {
                error!(
                    "Failed to load task {} after completion notification: {}",
                    task_id, err
                );
                return;
            }
        };

        if task.status != TaskStatus::Running {
            info!(
                "Ignoring completion notification for task {} in status {}",
                task_id, task.status
            );
            return;
        }

        if self.has_unfinished_subtasks(task_id) {
            info!(
                "Deferring completion for task {} because it still has unfinished subtasks",
                task_id
            );
            return;
        }

        info!("Task {} completed successfully", task_id);
        if let Err(err) = self.store.update_task_status(task_id, TaskStatus::Done) {
            error!(
                "Failed to update status to done for task {}: {}",
                task_id, err
            );
            return;
        }
        self.remove_task_dir(task_id);
    }

    fn handle_task_failed(&self, task_id: Uuid, error: BabataError) {
        self.running_tasks.lock().remove(&task_id);

        let task = match self.store.get_task(task_id) {
            Ok(task) => task,
            Err(store_error) => {
                error!(
                    "Failed to load task {} after failure notification: {}",
                    task_id, store_error
                );
                return;
            }
        };

        if task.status != TaskStatus::Running {
            info!(
                "Ignoring failure notification for task {} in status {}: {}",
                task_id, task.status, error
            );
            return;
        }

        warn!("Task {} failed and will be relaunched: {}", task_id, error);
        match self.launcher.launch(&task, self.exit_tx.clone()) {
            Ok(running_task) => {
                self.running_tasks.lock().insert(task_id, running_task);
            }
            Err(err) => {
                error!("Failed to relaunch task {} after failure: {}", task_id, err);
            }
        }
    }

    fn ensure_task_dir(&self, task_id: Uuid) -> BabataResult<()> {
        let task_dir = task_dir(task_id)?;
        std::fs::create_dir_all(&task_dir).map_err(|err| {
            BabataError::internal(format!(
                "Failed to create task directory '{}': {}",
                task_dir.display(),
                err
            ))
        })
    }

    fn remove_task_dir(&self, task_id: Uuid) {
        let task_dir = match task_dir(task_id) {
            Ok(path) => path,
            Err(err) => {
                error!(
                    "Failed to resolve task directory for task {} cleanup: {}",
                    task_id, err
                );
                return;
            }
        };

        if !task_dir.exists() {
            return;
        }

        if let Err(err) = std::fs::remove_dir_all(&task_dir) {
            error!(
                "Failed to remove task directory '{}' for task {}: {}",
                task_dir.display(),
                task_id,
                err
            );
        }
    }

    fn has_unfinished_subtasks(&self, task_id: Uuid) -> bool {
        match self.store.list_subtasks(task_id) {
            Ok(subtasks) => subtasks
                .into_iter()
                .any(|task| !matches!(task.status, TaskStatus::Done | TaskStatus::Canceled)),
            Err(err) => {
                error!(
                    "Failed to load subtasks for task {} while checking completion: {}",
                    task_id, err
                );
                false
            }
        }
    }
}

#[derive(Debug)]
pub struct RunningTask {
    pub task_id: Uuid,
    pub handle: JoinHandle<()>,
}
