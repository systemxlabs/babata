use std::{collections::HashMap, sync::Arc};

use chrono::Utc;
use futures::FutureExt;
use log::info;
use parking_lot::Mutex;
use tokio::{sync::mpsc, task::JoinHandle};
use uuid::Uuid;

const MAX_TASK_TREE_DEPTH: usize = 5;

use crate::{
    BabataResult,
    agent::Agent,
    error::BabataError,
    http::CollaborateTaskRequest,
    memory::MessageRecord,
    message::Content,
    task::{
        CollaborationTaskState, CreateTaskRequest, SteerMessage, TaskExitEvent, TaskRecord,
        TaskStatus, TaskStore, launcher::TaskLauncher,
    },
    task_error, task_info,
    utils::task_dir,
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

    /// Send a steer message to a running task.
    pub async fn steer_task(&self, task_id: Uuid, content: Vec<Content>) -> BabataResult<()> {
        // Check if target task exists and is running
        let target_task = self.store.get_task(task_id)?;
        if !matches!(target_task.status, TaskStatus::Running) {
            return Err(BabataError::invalid_input(format!(
                "Cannot steer task '{}': task is not running (status: {})",
                task_id, target_task.status
            )));
        }

        // Get the steer sender for the target task
        let sender = self
            .running_tasks
            .lock()
            .get(&task_id)
            .map(|task| task.steer_tx.clone())
            .ok_or_else(|| {
                BabataError::invalid_input(format!(
                    "Cannot steer task '{}': task is not running or steer channel not available",
                    task_id
                ))
            })?;

        // Send the steer message
        let steer_msg = SteerMessage::new(content);
        sender.send(steer_msg).await.map_err(|_| {
            BabataError::tool(format!(
                "Failed to send steer message to task '{}'",
                task_id
            ))
        })?;

        Ok(())
    }

    pub(crate) fn collaborate_task(
        &self,
        task_id: Uuid,
        request: CollaborateTaskRequest,
    ) -> BabataResult<()> {
        let task = self.store.get_task(task_id)?;
        if task.status != TaskStatus::Running {
            return Err(BabataError::invalid_input(format!(
                "Task '{}' cannot collaborate from status '{}'; only running tasks can collaborate",
                task_id, task.status
            )));
        }

        let mut running_tasks = self.running_tasks.lock();
        let running_task = running_tasks.get_mut(&task_id).ok_or_else(|| {
            BabataError::invalid_input(format!(
                "Cannot collaborate on task '{}': task is not running",
                task_id
            ))
        })?;

        if running_task
            .collaboration_handle
            .as_ref()
            .is_some_and(|handle| !handle.is_finished())
        {
            return Err(BabataError::invalid_input(format!(
                "Task '{}' already has a running collaboration task",
                task_id
            )));
        }

        if running_task.collaboration_handle.is_some() {
            let _ = running_task.collaboration_handle.take();
        }

        let collaboration_handle =
            self.launcher
                .collaborate(&task, &request.agent, &request.prompt)?;
        running_task.collaboration_handle = Some(collaboration_handle);

        Ok(())
    }

    pub fn get_collaboration_task_state(
        &self,
        task_id: Uuid,
    ) -> BabataResult<CollaborationTaskState> {
        let mut running_tasks = self.running_tasks.lock();
        let running_task = running_tasks.get_mut(&task_id).ok_or_else(|| {
            BabataError::not_found(format!("Running task '{}' not found", task_id))
        })?;

        let Some(handle) = running_task.collaboration_handle.as_ref() else {
            return Ok(CollaborationTaskState::NonExisting);
        };
        if !handle.is_finished() {
            return Ok(CollaborationTaskState::Running);
        }

        let handle = running_task.collaboration_handle.take().ok_or_else(|| {
            BabataError::internal("Finished collaboration handle missing from running task")
        })?;
        let result = handle.now_or_never().ok_or_else(|| {
            BabataError::internal("Finished collaboration task did not resolve immediately")
        })?;

        match result.map_err(|e| {
            BabataError::internal(format!("Collaboration task execution failed: {e}"))
        })? {
            Ok(content) => Ok(CollaborationTaskState::Succeed { result: content }),
            Err(e) => Ok(CollaborationTaskState::Failed {
                reason: format!("{e}"),
            }),
        }
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
            .list_tasks(Some(TaskStatus::Running), usize::MAX, None)?;
        if tasks.is_empty() {
            info!("No running tasks to recover on startup");
            return Ok(());
        }

        info!("Recovering {} running task(s) from task store", tasks.len());
        for task in tasks {
            if self.running_tasks.lock().contains_key(&task.task_id) {
                task_info!(
                    task.task_id,
                    "Skipping recovery because it is already running"
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
            task_info!(task.task_id, "Recovered running task");
        }

        Ok(())
    }

    pub fn create_task(&self, request: CreateTaskRequest) -> BabataResult<Uuid> {
        let task_id = Uuid::new_v4();
        task_info!(task_id, "Creating task with request: {:?}", request);

        let (root_task_id, parent_depth) = if let Some(parent_task_id) = request.parent_task_id {
            let task_record = self.store.get_task(parent_task_id)?;
            let depth = self.calculate_task_depth(parent_task_id)?;
            (task_record.root_task_id, depth)
        } else {
            (task_id, 0)
        };

        // Check task tree depth limit
        if parent_depth >= MAX_TASK_TREE_DEPTH {
            return Err(BabataError::invalid_input(format!(
                "Cannot create task: maximum task tree depth ({}) reached",
                MAX_TASK_TREE_DEPTH
            )));
        }

        let task_record = TaskRecord {
            task_id,
            description: request.description,
            agent: request.agent,
            status: TaskStatus::Running,
            parent_task_id: request.parent_task_id,
            root_task_id,
            created_at: Utc::now().timestamp_millis(),
            never_ends: request.never_ends,
        };
        self.store.insert_task(task_record.clone())?;

        let running_task =
            self.launcher
                .launch(&task_record, request.prompt, self.exit_tx.clone())?;
        {
            let mut guard = self.running_tasks.lock();
            guard.insert(task_id, running_task);
        }

        Ok(task_id)
    }

    pub fn pause_task(&self, task_id: Uuid) -> BabataResult<()> {
        task_info!(task_id, "Pausing task");
        let task = self.store.get_task(task_id)?;
        if task.status != TaskStatus::Running {
            return Err(BabataError::invalid_input(format!(
                "Task '{}' cannot be paused from status '{}'",
                task_id, task.status
            )));
        }

        if let Some(running_task) = self.running_tasks.lock().remove(&task_id) {
            running_task.abort();
        }

        self.store.update_task_status(task_id, TaskStatus::Paused)?;
        Ok(())
    }

    pub fn resume_task(&self, task_id: Uuid) -> BabataResult<()> {
        task_info!(task_id, "Resuming task");
        let task = self.store.get_task(task_id)?;
        if task.status != TaskStatus::Paused {
            return Err(BabataError::invalid_input(format!(
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

    pub fn cancel_task(&self, task_id: Uuid) -> BabataResult<()> {
        task_info!(task_id, "Cancelling task");
        let task = self.store.get_task(task_id)?;
        if task.status.is_terminal_status() {
            return Err(BabataError::invalid_input(format!(
                "Task '{}' cannot be canceled from status '{}'",
                task_id, task.status
            )));
        }

        self.cancel_task_recursive(task_id)?;
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

    /// List root tasks with pagination and filtering
    pub fn list_root_tasks(
        &self,
        status: Option<TaskStatus>,
        limit: usize,
        offset: usize,
    ) -> BabataResult<(Vec<TaskRecord>, usize)> {
        let tasks = self.store.list_root_tasks(status, limit, offset)?;
        let total = self.store.count_root_tasks(status)?;

        Ok((tasks, total))
    }

    /// Get children of a task
    pub fn get_task_children(&self, task_id: Uuid) -> BabataResult<Vec<TaskRecord>> {
        self.store.list_subtasks(task_id)
    }

    pub fn get_task(&self, task_id: Uuid) -> BabataResult<TaskRecord> {
        self.store.get_task(task_id)
    }

    pub fn get_task_messages(
        &self,
        task_id: Uuid,
        offset: usize,
        limit: usize,
    ) -> BabataResult<Vec<MessageRecord>> {
        let task = self.store.get_task(task_id)?;
        let memory = self.launcher.memories.get(&task.agent).ok_or_else(|| {
            BabataError::config(format!("Agent '{}' memory not found", task.agent))
        })?;

        memory.scan_task_message_records(task_id, offset, limit)
    }

    pub fn task_exists(&self, task_id: Uuid) -> BabataResult<bool> {
        self.store.task_exists(task_id)
    }

    pub fn count_tasks(&self, status: Option<TaskStatus>) -> BabataResult<usize> {
        self.store.count_tasks(status)
    }

    pub fn delete_task(&self, task_id: Uuid) -> BabataResult<()> {
        task_info!(task_id, "Deleting task");

        // Get all subtasks recursively
        let subtasks = self.store.list_all_subtasks(task_id)?;

        // Cancel and delete the target task if it's running
        if let Some(running_task) = self.running_tasks.lock().remove(&task_id) {
            running_task.abort();
        }

        // Delete subtasks: cancel running, delete metadata, delete directory
        for subtask in &subtasks {
            // Cancel if running
            if let Some(running_task) = self.running_tasks.lock().remove(&subtask.task_id) {
                running_task.abort();
            }
            // Delete from store
            if let Err(err) = self.store.delete_task(subtask.task_id) {
                task_error!(subtask.task_id, "Failed to delete subtask: {}", err);
            }
            // Delete task directory
            remove_task_dir(subtask.task_id)?;
        }

        // Delete the target task from store
        self.store.delete_task(task_id)?;
        // Delete the target task directory
        remove_task_dir(task_id)?;

        task_info!(task_id, "Deleted task and {} subtask(s)", subtasks.len());
        Ok(())
    }

    /// Calculate the depth of a task in the task tree.
    /// Root task has depth 1, its direct children have depth 2, etc.
    fn calculate_task_depth(&self, task_id: Uuid) -> BabataResult<usize> {
        let mut depth = 1;
        let mut current_id = task_id;

        loop {
            let task = self.store.get_task(current_id)?;
            match task.parent_task_id {
                Some(parent_id) => {
                    depth += 1;
                    current_id = parent_id;
                }
                None => break,
            }
        }

        Ok(depth)
    }

    fn handle_task_exit(&self, event: TaskExitEvent) {
        match event {
            TaskExitEvent::Completed { task_id } => self.handle_task_completed(task_id),
            TaskExitEvent::Failed { task_id, error } => self.handle_task_failed(task_id, error),
        }
    }

    fn handle_task_completed(&self, task_id: Uuid) {
        if let Some(running_task) = self.running_tasks.lock().remove(&task_id) {
            running_task.abort();
        }
        let task = match self.store.get_task(task_id) {
            Ok(task) => task,
            Err(err) => {
                task_error!(
                    task_id,
                    "Failed to load task after completion notification: {}",
                    err
                );
                return;
            }
        };

        if task.status != TaskStatus::Running {
            task_info!(
                task_id,
                "Ignoring completion notification in status {}",
                task.status
            );
            return;
        }

        if self.has_unfinished_subtasks(task_id) {
            let reason = format!(
                "Task {} is being relaunched because it attempted to finish while there are still unfinished subtasks. A parent task must remain running until all of its subtasks are completed, failed, or canceled.",
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

        task_info!(task_id, "Task completed successfully");
        if let Err(err) = self
            .store
            .update_task_status(task_id, TaskStatus::Completed)
        {
            task_error!(task_id, "Failed to update status to completed: {}", err);
        }
    }

    fn relaunch_after_completion(&self, task: &TaskRecord, reason: &str, failure_context: &str) {
        match self.launcher.relaunch(task, self.exit_tx.clone(), reason) {
            Ok(running_task) => {
                self.running_tasks.lock().insert(task.task_id, running_task);
            }
            Err(err) => {
                task_error!(
                    task.task_id,
                    "Failed to relaunch task after {}: {}",
                    failure_context,
                    err
                );
            }
        }
    }

    fn handle_task_failed(&self, task_id: Uuid, error: BabataError) {
        if let Some(running_task) = self.running_tasks.lock().remove(&task_id) {
            running_task.abort();
        }

        let task = match self.store.get_task(task_id) {
            Ok(task) => task,
            Err(store_error) => {
                task_error!(
                    task_id,
                    "Failed to load task after failure notification: {}",
                    store_error
                );
                return;
            }
        };

        if task.status != TaskStatus::Running {
            task_info!(
                task_id,
                "Ignoring failure notification in status {}: {}",
                task.status,
                error
            );
            return;
        }

        task_error!(task_id, "Task failed: {error}");
        if let Err(err) = self.store.update_task_status(task_id, TaskStatus::Failed) {
            task_error!(task_id, "Failed to update status to failed: {}", err);
        }
    }

    fn has_unfinished_subtasks(&self, task_id: Uuid) -> bool {
        match self.store.list_subtasks(task_id) {
            Ok(subtasks) => subtasks
                .into_iter()
                .any(|task| !task.status.is_terminal_status()),
            Err(err) => {
                task_error!(
                    task_id,
                    "Failed to load subtasks while checking completion: {}",
                    err
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

        if task.status.is_terminal_status() {
            return Ok(());
        }

        task_info!(task_id, "Cancelling task recursively");
        if let Some(running_task) = self.running_tasks.lock().remove(&task_id) {
            running_task.abort();
        }

        self.store
            .update_task_status(task_id, TaskStatus::Canceled)?;
        Ok(())
    }

    pub fn default_agent(&self) -> &Arc<Agent> {
        &self.launcher.default_agent
    }
}

#[derive(Debug)]
pub struct RunningTask {
    pub task_id: Uuid,
    pub handle: JoinHandle<()>,
    pub steer_tx: mpsc::Sender<SteerMessage>,
    pub collaboration_handle: Option<JoinHandle<BabataResult<Vec<Content>>>>,
}

impl RunningTask {
    fn abort(self) {
        self.handle.abort();
        if let Some(collaboration_handle) = self.collaboration_handle {
            collaboration_handle.abort();
        }
    }
}

fn remove_task_dir(task_id: Uuid) -> BabataResult<()> {
    let task_dir = task_dir(task_id)?;

    if !task_dir.exists() {
        return Ok(());
    }

    std::fs::remove_dir_all(&task_dir)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{Agent, AgentFrontmatter};
    use std::{collections::HashMap, fs, path::PathBuf};
    use uuid::Uuid;

    fn task_record(never_ends: bool) -> TaskRecord {
        let task_id = Uuid::new_v4();
        TaskRecord {
            task_id,
            description: "test task".to_string(),
            agent: "test-agent".to_string(),
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
            agent: "test-agent".to_string(),
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

    fn build_test_manager(temp_root: &std::path::Path) -> TaskManager {
        let _workspace = temp_root.join("workspace");
        fs::create_dir_all(&_workspace).expect("create workspace");

        // Create agent home directory and AGENT.md
        let agent_home = temp_root.join("agents").join("test-agent");
        fs::create_dir_all(&agent_home).expect("create agent home directory");
        let agent_md_path = agent_home.join("AGENT.md");
        fs::write(&agent_md_path, "---\nname: test-agent\n---\n").expect("create AGENT.md");

        let mut agents = HashMap::new();
        agents.insert(
            "test-agent".to_string(),
            Arc::new(Agent {
                path: agent_md_path,
                frontmatter: AgentFrontmatter {
                    name: "test-agent".to_string(),
                    description: "Test agent".to_string(),
                    provider: "openai".to_string(),
                    model: "gpt-4".to_string(),
                    allowed_tools: vec!["*".to_string()],
                    default: Some(true),
                },
                body: String::new(),
            }),
        );

        let store = TaskStore::open(temp_root.join("task.db")).expect("open temp task store");
        let launcher = TaskLauncher::new(agents, HashMap::new()).expect("build task launcher");
        TaskManager::new(store, launcher).expect("build task manager")
    }

    fn cleanup_task_artifacts(manager: &TaskManager, task_id: Uuid) {
        if let Some(running_task) = manager.running_tasks.lock().remove(&task_id) {
            running_task.abort();
        }
        let _ = remove_task_dir(task_id);
    }

    fn insert_dummy_running_task(
        manager: &TaskManager,
        task_id: Uuid,
    ) -> mpsc::Receiver<SteerMessage> {
        let (steer_tx, steer_rx) = mpsc::channel(1);
        manager.running_tasks.lock().insert(
            task_id,
            RunningTask {
                task_id,
                handle: tokio::spawn(async move {
                    std::future::pending::<()>().await;
                }),
                steer_tx,
                collaboration_handle: None,
            },
        );
        steer_rx
    }

    #[tokio::test]
    async fn handle_task_completed_relaunches_never_ending_task() {
        let temp_root = temp_test_root("manager-never-ends");
        fs::create_dir_all(&temp_root).expect("create temp root");
        let manager = build_test_manager(&temp_root);
        let task = task_record(true);
        manager
            .store
            .insert_task(task.clone())
            .expect("insert task record");

        manager.handle_task_completed(task.task_id);

        let stored_task = manager.store.get_task(task.task_id).expect("load task");
        assert_eq!(stored_task.status, TaskStatus::Running);
        assert!(manager.running_tasks.lock().contains_key(&task.task_id));

        cleanup_task_artifacts(&manager, task.task_id);
        let _ = fs::remove_dir_all(&temp_root);
    }

    #[tokio::test]
    async fn handle_task_completed_marks_root_task_completed() {
        let temp_root = temp_test_root("manager-complete-root");
        fs::create_dir_all(&temp_root).expect("create temp root");
        let manager = build_test_manager(&temp_root);
        let task = task_record(false);
        manager
            .store
            .insert_task(task.clone())
            .expect("insert task record");

        manager.handle_task_completed(task.task_id);

        let stored_task = manager.store.get_task(task.task_id).expect("load task");
        assert_eq!(stored_task.status, TaskStatus::Completed);
        let _ = fs::remove_dir_all(&temp_root);
    }

    #[tokio::test]
    async fn handle_task_completed_relaunches_when_subtasks_are_unfinished() {
        let temp_root = temp_test_root("manager-unfinished-subtasks");
        fs::create_dir_all(&temp_root).expect("create temp root");
        let manager = build_test_manager(&temp_root);
        let task = task_record(false);
        let subtask = subtask_record(task.task_id, task.root_task_id);
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

        cleanup_task_artifacts(&manager, task.task_id);
        let _ = fs::remove_dir_all(&temp_root);
    }

    #[tokio::test]
    async fn handle_task_failed_marks_task_failed_without_relaunch() {
        let temp_root = temp_test_root("manager-failure-no-relaunch");
        fs::create_dir_all(&temp_root).expect("create temp root");
        let manager = build_test_manager(&temp_root);
        let task = task_record(false);
        manager
            .store
            .insert_task(task.clone())
            .expect("insert task record");

        manager.handle_task_failed(task.task_id, BabataError::tool("boom"));

        let stored_task = manager.store.get_task(task.task_id).expect("load task");
        assert_eq!(stored_task.status, TaskStatus::Failed);
        assert!(!manager.running_tasks.lock().contains_key(&task.task_id));

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[tokio::test]
    async fn create_task_stores_steer_sender_in_running_task() {
        let temp_root = temp_test_root("manager-create-stores-steer");
        fs::create_dir_all(&temp_root).expect("create temp root");
        let manager = build_test_manager(&temp_root);
        let task_id = manager
            .create_task(CreateTaskRequest {
                description: "test create task".to_string(),
                prompt: vec![Content::Text {
                    text: "test create task".to_string(),
                }],
                parent_task_id: None,
                agent: "test-agent".to_string(),
                never_ends: false,
            })
            .expect("create task");

        {
            let guard = manager.running_tasks.lock();
            let running_task = guard.get(&task_id).expect("running task should exist");
            let _sender = running_task.steer_tx.clone();
        }

        cleanup_task_artifacts(&manager, task_id);
        let _ = manager.store.delete_task(task_id);
        let _ = fs::remove_dir_all(&temp_root);
    }

    #[tokio::test]
    async fn steer_task_sends_message_to_registered_sender() {
        let temp_root = temp_test_root("manager-steer-sends-message");
        fs::create_dir_all(&temp_root).expect("create temp root");
        let manager = build_test_manager(&temp_root);
        let task = task_record(false);
        manager
            .store
            .insert_task(task.clone())
            .expect("insert task record");

        let mut steer_rx = insert_dummy_running_task(&manager, task.task_id);

        manager
            .steer_task(
                task.task_id,
                vec![Content::Text {
                    text: "focus on tests".to_string(),
                }],
            )
            .await
            .expect("steer running task");

        let message = steer_rx.recv().await.expect("receive steer message");
        assert_eq!(
            message.content,
            vec![Content::Text {
                text: "focus on tests".to_string(),
            }]
        );

        cleanup_task_artifacts(&manager, task.task_id);
        let _ = fs::remove_dir_all(&temp_root);
    }

    #[tokio::test]
    async fn pause_task_removes_running_task_with_steer_sender() {
        let temp_root = temp_test_root("manager-pause-removes-running-task");
        fs::create_dir_all(&temp_root).expect("create temp root");
        let manager = build_test_manager(&temp_root);
        let task = task_record(false);
        manager
            .store
            .insert_task(task.clone())
            .expect("insert task record");

        drop(insert_dummy_running_task(&manager, task.task_id));

        manager.pause_task(task.task_id).expect("pause task");

        let stored_task = manager.store.get_task(task.task_id).expect("load task");
        assert_eq!(stored_task.status, TaskStatus::Paused);
        assert!(!manager.running_tasks.lock().contains_key(&task.task_id));

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[tokio::test]
    async fn delete_task_removes_running_tasks_for_root_and_subtasks() {
        let temp_root = temp_test_root("manager-delete-removes-running-tasks");
        fs::create_dir_all(&temp_root).expect("create temp root");
        let manager = build_test_manager(&temp_root);
        let task = task_record(false);
        let subtask = subtask_record(task.task_id, task.root_task_id);
        manager
            .store
            .insert_task(task.clone())
            .expect("insert root task");
        manager
            .store
            .insert_task(subtask.clone())
            .expect("insert subtask");

        drop(insert_dummy_running_task(&manager, task.task_id));
        drop(insert_dummy_running_task(&manager, subtask.task_id));

        manager.delete_task(task.task_id).expect("delete root task");

        assert!(!manager.running_tasks.lock().contains_key(&task.task_id));
        assert!(!manager.running_tasks.lock().contains_key(&subtask.task_id));
        assert!(manager.store.get_task(task.task_id).is_err());
        assert!(manager.store.get_task(subtask.task_id).is_err());

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[tokio::test]
    async fn delete_non_root_task_removes_itself_and_descendants() {
        let temp_root = temp_test_root("manager-delete-non-root-task");
        fs::create_dir_all(&temp_root).expect("create temp root");
        let manager = build_test_manager(&temp_root);
        let root_task = task_record(false);
        let mid_task = subtask_record(root_task.task_id, root_task.root_task_id);
        let leaf_task = subtask_record(mid_task.task_id, root_task.root_task_id);

        manager
            .store
            .insert_task(root_task.clone())
            .expect("insert root task");
        manager
            .store
            .insert_task(mid_task.clone())
            .expect("insert mid task");
        manager
            .store
            .insert_task(leaf_task.clone())
            .expect("insert leaf task");

        drop(insert_dummy_running_task(&manager, mid_task.task_id));
        drop(insert_dummy_running_task(&manager, leaf_task.task_id));

        manager
            .delete_task(mid_task.task_id)
            .expect("delete non-root task");

        // Root task should remain
        assert!(manager.store.get_task(root_task.task_id).is_ok());
        // Mid task and its descendants should be removed
        assert!(!manager.running_tasks.lock().contains_key(&mid_task.task_id));
        assert!(
            !manager
                .running_tasks
                .lock()
                .contains_key(&leaf_task.task_id)
        );
        assert!(manager.store.get_task(mid_task.task_id).is_err());
        assert!(manager.store.get_task(leaf_task.task_id).is_err());

        let _ = fs::remove_dir_all(&temp_root);
    }
}
