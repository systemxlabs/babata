use std::{collections::HashMap, sync::Arc};

use chrono::Utc;
use log::{error, info, warn};
use parking_lot::Mutex;
use tokio::{sync::mpsc, task::JoinHandle};
use uuid::Uuid;

use crate::{
    BabataResult,
    error::BabataError,
    message::Content,
    task::{
        CreateTaskRequest, TaskExitEvent, TaskRecord, TaskStatus, TaskStore,
        launcher::TaskLauncher, task_dir,
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
            .list_tasks(Some(TaskStatus::Running), 1000, None)?;
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

            let reason = format!(
                "Task {} is being relaunched to continue running when server started.",
                task.task_id
            );
            let running_task = self
                .launcher
                .relaunch(&task, self.exit_tx.clone(), &reason)?;
            self.running_tasks.lock().insert(task.task_id, running_task);
            info!("Recovered running task {}", task.task_id);
        }

        Ok(())
    }

    pub fn create_task(&self, request: CreateTaskRequest) -> BabataResult<Uuid> {
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
            description: render_prompt_markdown(&request.prompt),
            agent: request.agent.clone(),
            status: TaskStatus::Running,
            parent_task_id: request.parent_task_id,
            root_task_id,
            created_at: Utc::now().timestamp_millis(),
            never_ends: request.never_ends,
        };
        initialize_task_dir(&task_record, &request.prompt)?;
        self.store.insert_task(task_record.clone())?;

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

        let reason = format!(
            "Task {} is being relaunched because it was resumed from paused status by a user or system request.",
            task_id
        );
        let running_task = self
            .launcher
            .relaunch(&task, self.exit_tx.clone(), &reason)?;
        {
            let mut guard = self.running_tasks.lock();
            guard.insert(task_id, running_task);
        }

        self.store
            .update_task_status(task_id, TaskStatus::Running)?;
        Ok(())
    }

    pub fn relaunch_task(&self, task_id: Uuid, reason: &str) -> BabataResult<()> {
        let reason = reason.trim();
        if reason.is_empty() {
            return Err(BabataError::config("Relaunch reason cannot be empty"));
        }

        info!("Relaunching task {} with reason: {}", task_id, reason);
        let task = self.store.get_task(task_id)?;
        if task.status != TaskStatus::Running {
            return Err(BabataError::config(format!(
                "Task '{}' cannot be relaunched from status '{}'; only running tasks can be relaunched",
                task_id, task.status
            )));
        }

        if let Some(running_task) = self.running_tasks.lock().remove(&task_id) {
            running_task.handle.abort();
        }

        let running_task = self
            .launcher
            .relaunch(&task, self.exit_tx.clone(), reason)?;
        {
            let mut guard = self.running_tasks.lock();
            guard.insert(task_id, running_task);
        }

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

        self.cancel_task_recursive(task_id)?;
        if task.task_id == task.root_task_id {
            self.remove_task_dir_recursive(task_id);
        }
        Ok(())
    }

    pub fn list_tasks(
        &self,
        status: Option<TaskStatus>,
        limit: usize,
        offset: Option<usize>,
    ) -> BabataResult<Vec<TaskRecord>> {
        self.store.list_tasks(status, limit, offset)
    }

    pub fn get_task(&self, task_id: Uuid) -> BabataResult<TaskRecord> {
        self.store.get_task(task_id)
    }

    pub fn count_tasks(&self, status: Option<TaskStatus>) -> BabataResult<usize> {
        self.store.count_tasks(status)
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
            let reason = format!(
                "Task {} is being relaunched because it attempted to finish while there are still unfinished subtasks. A parent task must remain running until all of its subtasks are done or canceled.",
                task.task_id
            );
            self.relaunch_after_completion(&task, &reason, "deferred completion");
            return;
        }

        if task.never_ends {
            let reason = format!(
                "Task {} is being relaunched because it is configured with never_ends=true and should keep running after reporting completion.",
                task.task_id
            );
            self.relaunch_after_completion(&task, &reason, "never-ending completion");
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
        if task.task_id == task.root_task_id {
            self.remove_task_dir_recursive(task_id);
        }
    }

    fn relaunch_after_completion(&self, task: &TaskRecord, reason: &str, failure_context: &str) {
        info!("{reason}");
        match self.launcher.relaunch(task, self.exit_tx.clone(), reason) {
            Ok(running_task) => {
                self.running_tasks.lock().insert(task.task_id, running_task);
            }
            Err(err) => {
                error!(
                    "Failed to relaunch task {} after {}: {}",
                    task.task_id, failure_context, err
                );
            }
        }
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
        let reason = format!(
            "Task {} is being relaunched after the previous execution failed with error: {}",
            task_id, error
        );
        match self.launcher.relaunch(&task, self.exit_tx.clone(), &reason) {
            Ok(running_task) => {
                self.running_tasks.lock().insert(task_id, running_task);
            }
            Err(err) => {
                error!("Failed to relaunch task {} after failure: {}", task_id, err);
            }
        }
    }

    fn remove_task_dir_recursive(&self, task_id: Uuid) {
        let subtasks = match self.store.list_subtasks(task_id) {
            Ok(subtasks) => subtasks,
            Err(err) => {
                error!(
                    "Failed to load subtasks for task {} while cleaning task tree directories: {}",
                    task_id, err
                );
                return;
            }
        };

        for subtask in subtasks {
            self.remove_task_dir_recursive(subtask.task_id);
        }

        remove_task_dir(task_id);
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

    fn cancel_task_recursive(&self, task_id: Uuid) -> BabataResult<()> {
        let task = self.store.get_task(task_id)?;
        let subtasks = self.store.list_subtasks(task_id)?;

        for subtask in subtasks {
            self.cancel_task_recursive(subtask.task_id)?;
        }

        if matches!(task.status, TaskStatus::Done | TaskStatus::Canceled) {
            return Ok(());
        }

        info!("Cancelling task {} recursively", task_id);
        if let Some(running_task) = self.running_tasks.lock().remove(&task_id) {
            running_task.handle.abort();
        }

        self.store
            .update_task_status(task_id, TaskStatus::Canceled)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct RunningTask {
    pub task_id: Uuid,
    pub handle: JoinHandle<()>,
}

fn ensure_task_dir(task_id: Uuid) -> BabataResult<()> {
    let task_dir = task_dir(task_id)?;
    std::fs::create_dir_all(&task_dir).map_err(|err| {
        BabataError::internal(format!(
            "Failed to create task directory '{}': {}",
            task_dir.display(),
            err
        ))
    })
}

fn initialize_task_dir(task: &TaskRecord, prompt: &[Content]) -> BabataResult<()> {
    ensure_task_dir(task.task_id)?;

    let task_dir = task_dir(task.task_id)?;
    let task_md_path = task_dir.join("task.md");
    let progress_md_path = task_dir.join("progress.md");
    let prompt = render_prompt_markdown(prompt);
    let agent = task.agent.as_deref().unwrap_or("babata");
    let task_markdown = format!(
        r#"# Task

## Metadata
- Task ID: {}
- Root Task ID: {}
- Parent Task ID: {}
- Agent: {}
- Status: {}
- Never Ends: {}

## Initial Prompt
{}
"#,
        task.task_id,
        task.root_task_id,
        task.parent_task_id
            .map(|task_id| task_id.to_string())
            .unwrap_or_else(|| "none".to_string()),
        agent,
        task.status,
        task.never_ends,
        prompt
    );
    let progress_markdown = r#"# Progress

- Status: running
- Updates: task created
"#
    .to_string();

    std::fs::write(&task_md_path, task_markdown).map_err(|err| {
        BabataError::internal(format!(
            "Failed to write task file '{}': {}",
            task_md_path.display(),
            err
        ))
    })?;
    std::fs::write(&progress_md_path, progress_markdown).map_err(|err| {
        BabataError::internal(format!(
            "Failed to write progress file '{}': {}",
            progress_md_path.display(),
            err
        ))
    })?;
    Ok(())
}

fn remove_task_dir(task_id: Uuid) {
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

fn render_prompt_markdown(prompt: &[Content]) -> String {
    let lines = prompt
        .iter()
        .map(|content| match content {
            Content::Text { text } => text.clone(),
            Content::ImageUrl { url } => format!("- [image] {url}"),
            Content::ImageData { media_type, .. } => format!("- [image_data] {media_type}"),
            Content::AudioData { media_type, .. } => format!("- [audio_data] {media_type}"),
        })
        .collect::<Vec<_>>();

    if lines.is_empty() {
        "_No prompt provided._".to_string()
    } else {
        lines.join("\n\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AgentConfig, CodexAgentConfig, Config};
    use std::{collections::HashMap, fs, path::PathBuf};
    use uuid::Uuid;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    fn task_record(never_ends: bool) -> TaskRecord {
        let task_id = Uuid::new_v4();
        TaskRecord {
            task_id,
            description: "test task".to_string(),
            agent: Some("codex".to_string()),
            status: TaskStatus::Running,
            parent_task_id: None,
            root_task_id: task_id,
            created_at: 123,
            never_ends,
        }
    }

    fn subtask_record(parent_task_id: Uuid, root_task_id: Uuid) -> TaskRecord {
        TaskRecord {
            task_id: Uuid::new_v4(),
            description: "test subtask".to_string(),
            agent: Some("codex".to_string()),
            status: TaskStatus::Running,
            parent_task_id: Some(parent_task_id),
            root_task_id,
            created_at: 123,
            never_ends: false,
        }
    }

    fn temp_test_root(test_name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("babata-{test_name}-{}", Uuid::new_v4()))
    }

    fn create_dummy_codex_command(dir: &std::path::Path) -> PathBuf {
        #[cfg(windows)]
        {
            let command_path = dir.join("fake-codex.cmd");
            fs::write(&command_path, "@echo off\r\nexit /b 0\r\n").expect("write fake codex cmd");
            command_path
        }

        #[cfg(not(windows))]
        {
            let command_path = dir.join("fake-codex");
            fs::write(&command_path, "#!/bin/sh\nexit 0\n").expect("write fake codex script");
            let mut permissions = fs::metadata(&command_path)
                .expect("read fake codex metadata")
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&command_path, permissions).expect("chmod fake codex script");
            command_path
        }
    }

    fn build_test_manager(temp_root: &std::path::Path) -> TaskManager {
        let workspace = temp_root.join("workspace");
        fs::create_dir_all(&workspace).expect("create workspace");

        let command = create_dummy_codex_command(temp_root);
        let config = Config {
            providers: Vec::new(),
            agents: vec![AgentConfig::Codex(CodexAgentConfig {
                command: command.display().to_string(),
                workspace: workspace.display().to_string(),
                model: None,
            })],
            channels: Vec::new(),
            memory: Vec::new(),
        };

        let store = TaskStore::open(temp_root.join("task.db")).expect("open temp task store");
        let launcher = TaskLauncher::new(&config, HashMap::new()).expect("build task launcher");
        TaskManager::new(store, launcher).expect("build task manager")
    }

    fn cleanup_task_artifacts(manager: &TaskManager, task_id: Uuid) {
        if let Some(running_task) = manager.running_tasks.lock().remove(&task_id) {
            running_task.handle.abort();
        }
        remove_task_dir(task_id);
    }

    #[tokio::test]
    async fn handle_task_completed_relaunches_never_ending_task() {
        let temp_root = temp_test_root("manager-never-ends");
        fs::create_dir_all(&temp_root).expect("create temp root");
        let manager = build_test_manager(&temp_root);
        let task = task_record(true);
        initialize_task_dir(
            &task,
            &[Content::Text {
                text: "keep running".to_string(),
            }],
        )
        .expect("initialize task dir");
        manager
            .store
            .insert_task(task.clone())
            .expect("insert task record");

        manager.handle_task_completed(task.task_id);

        let stored_task = manager.store.get_task(task.task_id).expect("load task");
        assert_eq!(stored_task.status, TaskStatus::Running);
        assert!(manager.running_tasks.lock().contains_key(&task.task_id));
        assert!(task_dir(task.task_id).expect("resolve task dir").exists());

        cleanup_task_artifacts(&manager, task.task_id);
        let _ = fs::remove_dir_all(&temp_root);
    }

    #[tokio::test]
    async fn handle_task_completed_marks_root_task_done_and_cleans_directory() {
        let temp_root = temp_test_root("manager-complete-root");
        fs::create_dir_all(&temp_root).expect("create temp root");
        let manager = build_test_manager(&temp_root);
        let task = task_record(false);
        initialize_task_dir(
            &task,
            &[Content::Text {
                text: "finish task".to_string(),
            }],
        )
        .expect("initialize task dir");
        manager
            .store
            .insert_task(task.clone())
            .expect("insert task record");

        manager.handle_task_completed(task.task_id);

        let stored_task = manager.store.get_task(task.task_id).expect("load task");
        assert_eq!(stored_task.status, TaskStatus::Done);
        assert!(!task_dir(task.task_id).expect("resolve task dir").exists());

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[tokio::test]
    async fn handle_task_completed_relaunches_when_subtasks_are_unfinished() {
        let temp_root = temp_test_root("manager-unfinished-subtasks");
        fs::create_dir_all(&temp_root).expect("create temp root");
        let manager = build_test_manager(&temp_root);
        let task = task_record(false);
        let subtask = subtask_record(task.task_id, task.root_task_id);
        initialize_task_dir(
            &task,
            &[Content::Text {
                text: "wait for subtask".to_string(),
            }],
        )
        .expect("initialize task dir");
        initialize_task_dir(
            &subtask,
            &[Content::Text {
                text: "subtask still running".to_string(),
            }],
        )
        .expect("initialize subtask dir");
        manager
            .store
            .insert_task(task.clone())
            .expect("insert parent task record");
        manager
            .store
            .insert_task(subtask.clone())
            .expect("insert subtask record");

        manager.handle_task_completed(task.task_id);

        let stored_task = manager
            .store
            .get_task(task.task_id)
            .expect("load parent task");
        assert_eq!(stored_task.status, TaskStatus::Running);
        assert!(manager.running_tasks.lock().contains_key(&task.task_id));
        assert!(
            task_dir(task.task_id)
                .expect("resolve parent task dir")
                .exists()
        );

        cleanup_task_artifacts(&manager, task.task_id);
        remove_task_dir(subtask.task_id);
        let _ = fs::remove_dir_all(&temp_root);
    }
}
