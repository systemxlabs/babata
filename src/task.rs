use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};

use crate::{BabataResult, error::BabataError, message::Message, utils::babata_dir};

pub const TASK_FILE_NAME: &str = "task.md";
pub const PROGRESS_FILE_NAME: &str = "progress.md";
pub const ARTIFACTS_DIR_NAME: &str = "artifacts";
pub const HISTORY_FILE_NAME: &str = "message_history.json";
pub const FINAL_OUTPUT_FILE_NAME: &str = "final_output.md";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Running,
    Done,
    Canceled,
    Paused,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Done => "done",
            Self::Canceled => "canceled",
            Self::Paused => "paused",
        }
    }

    pub fn parse(value: &str) -> BabataResult<Self> {
        match value {
            "running" => Ok(Self::Running),
            "done" => Ok(Self::Done),
            "canceled" => Ok(Self::Canceled),
            "paused" => Ok(Self::Paused),
            _ => Err(BabataError::internal(format!(
                "Unsupported task status '{}'",
                value
            ))),
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Done | Self::Canceled)
    }
}

#[derive(Debug, Clone)]
pub struct NewTask {
    pub agent_name: String,
    pub provider_name: String,
    pub model: String,
    pub task_markdown: String,
    pub initial_progress: String,
    pub initial_history: Vec<Message>,
    pub parent_task_id: Option<String>,
    pub root_task_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRecord {
    pub task_id: String,
    pub agent_name: String,
    pub provider_name: String,
    pub model: String,
    pub status: TaskStatus,
    pub parent_task_id: Option<String>,
    pub root_task_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub final_output: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactEntry {
    pub relative_path: String,
}

#[derive(Debug, Clone)]
pub struct TaskSnapshot {
    pub record: TaskRecord,
    pub task_markdown: String,
    pub progress_markdown: String,
    pub history: Vec<Message>,
    pub artifacts: Vec<ArtifactEntry>,
}

#[derive(Debug, Clone)]
pub struct TaskStore {
    base_dir: PathBuf,
    db_path: PathBuf,
    tasks_dir: PathBuf,
}

impl TaskStore {
    pub fn open_default() -> BabataResult<Self> {
        Self::new(babata_dir()?)
    }

    pub fn new(base_dir: impl AsRef<Path>) -> BabataResult<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();
        let db_path = base_dir.join("task.db");
        let tasks_dir = base_dir.join("tasks");
        let store = Self {
            base_dir,
            db_path,
            tasks_dir,
        };
        store.ensure_layout()?;
        Ok(store)
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn task_dir(&self, task_id: &str) -> PathBuf {
        self.tasks_dir.join(task_id)
    }

    pub fn task_path(&self, task_id: &str) -> PathBuf {
        self.task_dir(task_id).join(TASK_FILE_NAME)
    }

    pub fn progress_path(&self, task_id: &str) -> PathBuf {
        self.task_dir(task_id).join(PROGRESS_FILE_NAME)
    }

    pub fn artifacts_dir(&self, task_id: &str) -> PathBuf {
        self.task_dir(task_id).join(ARTIFACTS_DIR_NAME)
    }

    pub fn history_path(&self, task_id: &str) -> PathBuf {
        self.artifacts_dir(task_id).join(HISTORY_FILE_NAME)
    }

    pub fn final_output_path(&self, task_id: &str) -> PathBuf {
        self.artifacts_dir(task_id).join(FINAL_OUTPUT_FILE_NAME)
    }

    pub fn create_task(&self, new_task: NewTask) -> BabataResult<TaskRecord> {
        let task_id = uuid::Uuid::new_v4().to_string();
        let now = now_rfc3339();
        let root_task_id = new_task
            .root_task_id
            .clone()
            .unwrap_or_else(|| task_id.clone());

        let task_dir = self.task_dir(&task_id);
        fs::create_dir_all(self.artifacts_dir(&task_id)).map_err(|err| {
            BabataError::internal(format!(
                "Failed to create task directory '{}': {}",
                task_dir.display(),
                err
            ))
        })?;
        self.write_string(self.task_path(&task_id), &new_task.task_markdown)?;
        self.write_string(self.progress_path(&task_id), &new_task.initial_progress)?;
        self.write_history_messages(&task_id, &new_task.initial_history)?;

        let record = TaskRecord {
            task_id: task_id.clone(),
            agent_name: new_task.agent_name,
            provider_name: new_task.provider_name,
            model: new_task.model,
            status: TaskStatus::Running,
            parent_task_id: new_task.parent_task_id,
            root_task_id,
            created_at: now.clone(),
            updated_at: now,
            completed_at: None,
            final_output: None,
            last_error: None,
        };

        let conn = self.connect()?;
        conn.execute(
            r#"
            INSERT INTO tasks (
                task_id,
                agent_name,
                provider_name,
                model,
                status,
                parent_task_id,
                root_task_id,
                created_at,
                updated_at,
                completed_at,
                final_output,
                last_error
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, NULL, NULL)
            "#,
            params![
                record.task_id,
                record.agent_name,
                record.provider_name,
                record.model,
                record.status.as_str(),
                record.parent_task_id,
                record.root_task_id,
                record.created_at,
                record.updated_at,
            ],
        )
        .map_err(|err| BabataError::internal(format!("Failed to insert task record: {err}")))?;

        self.get_task(&task_id)
    }

    pub fn get_task(&self, task_id: &str) -> BabataResult<TaskRecord> {
        let conn = self.connect()?;
        let raw = conn
            .query_row(
                r#"
                SELECT
                    task_id,
                    agent_name,
                    provider_name,
                    model,
                    status,
                    parent_task_id,
                    root_task_id,
                    created_at,
                    updated_at,
                    completed_at,
                    final_output,
                    last_error
                FROM tasks
                WHERE task_id = ?
                "#,
                params![task_id],
                map_raw_task_row,
            )
            .optional()
            .map_err(|err| {
                BabataError::internal(format!("Failed to query task '{}': {}", task_id, err))
            })?;

        let Some(raw) = raw else {
            return Err(BabataError::internal(format!(
                "Task '{}' not found",
                task_id
            )));
        };

        raw.try_into()
    }

    pub fn list_tasks_by_status(&self, status: TaskStatus) -> BabataResult<Vec<TaskRecord>> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                r#"
                SELECT
                    task_id,
                    agent_name,
                    provider_name,
                    model,
                    status,
                    parent_task_id,
                    root_task_id,
                    created_at,
                    updated_at,
                    completed_at,
                    final_output,
                    last_error
                FROM tasks
                WHERE status = ?
                ORDER BY updated_at ASC, created_at ASC
                "#,
            )
            .map_err(|err| BabataError::internal(format!("Failed to prepare task query: {err}")))?;

        let rows = stmt
            .query_map(params![status.as_str()], map_raw_task_row)
            .map_err(|err| {
                BabataError::internal(format!(
                    "Failed to query tasks by status '{}': {}",
                    status.as_str(),
                    err
                ))
            })?;

        let mut tasks = Vec::new();
        for row in rows {
            let raw = row.map_err(|err| {
                BabataError::internal(format!("Failed to deserialize task row: {err}"))
            })?;
            tasks.push(raw.try_into()?);
        }
        Ok(tasks)
    }

    pub fn list_tasks(&self) -> BabataResult<Vec<TaskRecord>> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                r#"
                SELECT
                    task_id,
                    agent_name,
                    provider_name,
                    model,
                    status,
                    parent_task_id,
                    root_task_id,
                    created_at,
                    updated_at,
                    completed_at,
                    final_output,
                    last_error
                FROM tasks
                ORDER BY updated_at DESC, created_at DESC
                "#,
            )
            .map_err(|err| {
                BabataError::internal(format!("Failed to prepare list tasks query: {err}"))
            })?;

        let rows = stmt
            .query_map([], map_raw_task_row)
            .map_err(|err| BabataError::internal(format!("Failed to list tasks: {}", err)))?;

        let mut tasks = Vec::new();
        for row in rows {
            let raw = row.map_err(|err| {
                BabataError::internal(format!("Failed to deserialize task row: {err}"))
            })?;
            tasks.push(raw.try_into()?);
        }
        Ok(tasks)
    }

    pub fn set_status(
        &self,
        task_id: &str,
        status: TaskStatus,
        final_output: Option<&str>,
        last_error: Option<&str>,
    ) -> BabataResult<()> {
        let now = now_rfc3339();
        let completed_at = status.is_terminal().then_some(now.as_str());
        let conn = self.connect()?;
        conn.execute(
            r#"
            UPDATE tasks
            SET
                status = ?,
                updated_at = ?,
                completed_at = ?,
                final_output = COALESCE(?, final_output),
                last_error = ?
            WHERE task_id = ?
            "#,
            params![
                status.as_str(),
                now,
                completed_at,
                final_output,
                last_error,
                task_id,
            ],
        )
        .map_err(|err| {
            BabataError::internal(format!(
                "Failed to update status for task '{}': {}",
                task_id, err
            ))
        })?;
        Ok(())
    }

    pub fn record_error(&self, task_id: &str, last_error: Option<&str>) -> BabataResult<()> {
        let now = now_rfc3339();
        let conn = self.connect()?;
        conn.execute(
            "UPDATE tasks SET updated_at = ?, last_error = ? WHERE task_id = ?",
            params![now, last_error, task_id],
        )
        .map_err(|err| {
            BabataError::internal(format!(
                "Failed to record task error '{}': {}",
                task_id, err
            ))
        })?;
        Ok(())
    }

    pub fn load_snapshot(&self, task_id: &str) -> BabataResult<TaskSnapshot> {
        let record = self.get_task(task_id)?;
        let task_markdown = self.read_string(self.task_path(task_id))?;
        let progress_markdown = self.read_optional_string(self.progress_path(task_id))?;
        let history = self.read_history_messages(task_id)?;
        let artifacts = self.list_artifacts(task_id)?;
        Ok(TaskSnapshot {
            record,
            task_markdown,
            progress_markdown,
            history,
            artifacts,
        })
    }

    pub fn append_history_messages(&self, task_id: &str, messages: &[Message]) -> BabataResult<()> {
        let mut history = self.read_history_messages(task_id)?;
        history.extend(messages.iter().cloned());
        self.write_history_messages(task_id, &history)?;
        self.record_error(task_id, None)
    }

    pub fn write_progress_markdown(&self, task_id: &str, content: &str) -> BabataResult<()> {
        self.write_string(self.progress_path(task_id), content)?;
        self.record_error(task_id, None)
    }

    pub fn write_final_output(&self, task_id: &str, content: &str) -> BabataResult<()> {
        self.write_string(self.final_output_path(task_id), content)
    }

    pub fn read_final_output(&self, task_id: &str) -> BabataResult<Option<String>> {
        let path = self.final_output_path(task_id);
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(self.read_string(path)?))
    }

    pub fn list_artifacts(&self, task_id: &str) -> BabataResult<Vec<ArtifactEntry>> {
        let artifacts_dir = self.artifacts_dir(task_id);
        if !artifacts_dir.exists() {
            return Ok(Vec::new());
        }

        let mut entries = Vec::new();
        collect_artifact_entries(&artifacts_dir, &artifacts_dir, &mut entries)?;
        entries.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        Ok(entries)
    }

    fn ensure_layout(&self) -> BabataResult<()> {
        fs::create_dir_all(&self.tasks_dir).map_err(|err| {
            BabataError::internal(format!(
                "Failed to create tasks directory '{}': {}",
                self.tasks_dir.display(),
                err
            ))
        })?;

        let conn = self.connect()?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS tasks (
                task_id TEXT PRIMARY KEY,
                agent_name TEXT NOT NULL,
                provider_name TEXT NOT NULL,
                model TEXT NOT NULL,
                status TEXT NOT NULL,
                parent_task_id TEXT,
                root_task_id TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                completed_at TEXT,
                final_output TEXT,
                last_error TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_tasks_status_updated_at
            ON tasks(status, updated_at);
            "#,
        )
        .map_err(|err| {
            BabataError::internal(format!(
                "Failed to initialize task database '{}': {}",
                self.db_path.display(),
                err
            ))
        })?;

        Ok(())
    }

    fn connect(&self) -> BabataResult<Connection> {
        let conn = Connection::open(&self.db_path).map_err(|err| {
            BabataError::internal(format!(
                "Failed to open task database '{}': {}",
                self.db_path.display(),
                err
            ))
        })?;
        conn.busy_timeout(Duration::from_secs(5)).map_err(|err| {
            BabataError::internal(format!(
                "Failed to set SQLite busy timeout for '{}': {}",
                self.db_path.display(),
                err
            ))
        })?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|err| {
                BabataError::internal(format!(
                    "Failed to enable WAL mode for '{}': {}",
                    self.db_path.display(),
                    err
                ))
            })?;
        Ok(conn)
    }

    fn write_string(&self, path: PathBuf, content: &str) -> BabataResult<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                BabataError::internal(format!(
                    "Failed to create parent directory '{}': {}",
                    parent.display(),
                    err
                ))
            })?;
        }

        fs::write(&path, content).map_err(|err| {
            BabataError::internal(format!(
                "Failed to write file '{}': {}",
                path.display(),
                err
            ))
        })?;
        Ok(())
    }

    fn read_string(&self, path: PathBuf) -> BabataResult<String> {
        fs::read_to_string(&path).map_err(|err| {
            BabataError::internal(format!("Failed to read file '{}': {}", path.display(), err))
        })
    }

    fn read_optional_string(&self, path: PathBuf) -> BabataResult<String> {
        if !path.exists() {
            return Ok(String::new());
        }
        self.read_string(path)
    }

    fn read_history_messages(&self, task_id: &str) -> BabataResult<Vec<Message>> {
        let history_path = self.history_path(task_id);
        if !history_path.exists() {
            return Ok(Vec::new());
        }

        let raw = self.read_string(history_path)?;
        serde_json::from_str::<Vec<Message>>(&raw).map_err(|err| {
            BabataError::internal(format!(
                "Failed to parse task message history for '{}': {}",
                task_id, err
            ))
        })
    }

    fn write_history_messages(&self, task_id: &str, history: &[Message]) -> BabataResult<()> {
        let content = serde_json::to_string_pretty(history).map_err(|err| {
            BabataError::internal(format!(
                "Failed to serialize task history for '{}': {}",
                task_id, err
            ))
        })?;
        self.write_string(self.history_path(task_id), &content)
    }
}

#[derive(Debug)]
struct RawTaskRow {
    task_id: String,
    agent_name: String,
    provider_name: String,
    model: String,
    status: String,
    parent_task_id: Option<String>,
    root_task_id: String,
    created_at: String,
    updated_at: String,
    completed_at: Option<String>,
    final_output: Option<String>,
    last_error: Option<String>,
}

impl TryFrom<RawTaskRow> for TaskRecord {
    type Error = crate::error::BabataError;

    fn try_from(value: RawTaskRow) -> Result<Self, Self::Error> {
        Ok(TaskRecord {
            task_id: value.task_id,
            agent_name: value.agent_name,
            provider_name: value.provider_name,
            model: value.model,
            status: TaskStatus::parse(&value.status)?,
            parent_task_id: value.parent_task_id,
            root_task_id: value.root_task_id,
            created_at: value.created_at,
            updated_at: value.updated_at,
            completed_at: value.completed_at,
            final_output: value.final_output,
            last_error: value.last_error,
        })
    }
}

fn map_raw_task_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawTaskRow> {
    Ok(RawTaskRow {
        task_id: row.get(0)?,
        agent_name: row.get(1)?,
        provider_name: row.get(2)?,
        model: row.get(3)?,
        status: row.get(4)?,
        parent_task_id: row.get(5)?,
        root_task_id: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
        completed_at: row.get(9)?,
        final_output: row.get(10)?,
        last_error: row.get(11)?,
    })
}

fn collect_artifact_entries(
    root: &Path,
    dir: &Path,
    entries: &mut Vec<ArtifactEntry>,
) -> BabataResult<()> {
    let children = fs::read_dir(dir).map_err(|err| {
        BabataError::internal(format!(
            "Failed to read artifact directory '{}': {}",
            dir.display(),
            err
        ))
    })?;

    for child in children {
        let child = child.map_err(|err| {
            BabataError::internal(format!(
                "Failed to read artifact entry in '{}': {}",
                dir.display(),
                err
            ))
        })?;
        let path = child.path();
        if path.is_dir() {
            collect_artifact_entries(root, &path, entries)?;
            continue;
        }

        let relative_path = path
            .strip_prefix(root)
            .map_err(|err| {
                BabataError::internal(format!(
                    "Failed to resolve artifact path '{}' relative to '{}': {}",
                    path.display(),
                    root.display(),
                    err
                ))
            })?
            .to_string_lossy()
            .replace('\\', "/");
        entries.push(ArtifactEntry { relative_path });
    }

    Ok(())
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::message::{Content, Message};

    use super::{NewTask, TaskStatus, TaskStore};

    fn temp_store() -> TaskStore {
        let base = std::env::temp_dir().join(format!("babata-task-store-{}", uuid::Uuid::new_v4()));
        TaskStore::new(base).expect("create task store")
    }

    #[test]
    fn create_task_initializes_v2_layout() {
        let store = temp_store();
        let record = store
            .create_task(NewTask {
                agent_name: "main".to_string(),
                provider_name: "openai".to_string(),
                model: "gpt-4.1".to_string(),
                task_markdown: "# Task".to_string(),
                initial_progress: "# Progress".to_string(),
                initial_history: vec![Message::UserPrompt {
                    content: vec![Content::Text {
                        text: "hello".to_string(),
                    }],
                }],
                parent_task_id: None,
                root_task_id: None,
            })
            .expect("create task");

        assert_eq!(record.status, TaskStatus::Running);
        assert!(store.task_path(&record.task_id).exists());
        assert!(store.progress_path(&record.task_id).exists());
        assert!(store.history_path(&record.task_id).exists());
        assert!(store.task_dir(&record.task_id).exists());

        fs::remove_dir_all(store.base_dir()).expect("cleanup temp task store");
    }

    #[test]
    fn set_status_persists_final_output_and_terminal_state() {
        let store = temp_store();
        let record = store
            .create_task(NewTask {
                agent_name: "main".to_string(),
                provider_name: "openai".to_string(),
                model: "gpt-4.1".to_string(),
                task_markdown: "# Task".to_string(),
                initial_progress: "# Progress".to_string(),
                initial_history: Vec::new(),
                parent_task_id: None,
                root_task_id: None,
            })
            .expect("create task");

        store
            .write_final_output(&record.task_id, "done")
            .expect("write final output");
        store
            .set_status(&record.task_id, TaskStatus::Done, Some("done"), None)
            .expect("mark task done");

        let updated = store.get_task(&record.task_id).expect("reload task");
        assert_eq!(updated.status, TaskStatus::Done);
        assert_eq!(updated.final_output.as_deref(), Some("done"));
        assert!(updated.completed_at.is_some());

        fs::remove_dir_all(store.base_dir()).expect("cleanup temp task store");
    }

    #[test]
    fn list_tasks_filters_by_status() {
        let store = temp_store();
        let first = store
            .create_task(NewTask {
                agent_name: "main".to_string(),
                provider_name: "openai".to_string(),
                model: "gpt-4.1".to_string(),
                task_markdown: "# Task A".to_string(),
                initial_progress: "# Progress".to_string(),
                initial_history: Vec::new(),
                parent_task_id: None,
                root_task_id: None,
            })
            .expect("create first task");
        let second = store
            .create_task(NewTask {
                agent_name: "main".to_string(),
                provider_name: "openai".to_string(),
                model: "gpt-4.1".to_string(),
                task_markdown: "# Task B".to_string(),
                initial_progress: "# Progress".to_string(),
                initial_history: Vec::new(),
                parent_task_id: None,
                root_task_id: None,
            })
            .expect("create second task");

        store
            .set_status(&second.task_id, TaskStatus::Paused, None, Some("waiting"))
            .expect("pause second task");

        let all_tasks = store.list_tasks().expect("list all tasks");
        let running_tasks = store
            .list_tasks_by_status(TaskStatus::Running)
            .expect("list running tasks");

        assert_eq!(all_tasks.len(), 2);
        assert_eq!(running_tasks.len(), 1);
        assert_eq!(running_tasks[0].task_id, first.task_id);

        fs::remove_dir_all(store.base_dir()).expect("cleanup temp task store");
    }
}
