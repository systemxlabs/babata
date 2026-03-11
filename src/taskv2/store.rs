use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};
use uuid::Uuid;

use crate::{
    BabataResult,
    error::BabataError,
    taskv2::TaskStatus,
    utils::babata_dir,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRecord {
    pub task_id: Uuid,
    pub status: TaskStatus,
    pub parent_task_id: Option<Uuid>,
    pub root_task_id: Uuid,
}

#[derive(Debug)]
pub struct TaskStore {
    db_path: PathBuf,
}

impl TaskStore {
    pub fn new() -> BabataResult<Self> {
        Self::open(Self::default_db_path()?)
    }

    fn open(db_path: impl AsRef<Path>) -> BabataResult<Self> {
        let db_path = db_path.as_ref().to_path_buf();
        let Some(parent) = db_path.parent() else {
            return Err(BabataError::internal(format!(
                "Invalid task sqlite path '{}'",
                db_path.display()
            )));
        };

        std::fs::create_dir_all(parent).map_err(|err| {
            BabataError::internal(format!(
                "Failed to create task db directory '{}': {}",
                parent.display(),
                err
            ))
        })?;

        let conn = Connection::open(&db_path).map_err(|err| {
            BabataError::internal(format!(
                "Failed to open task db '{}': {}",
                db_path.display(),
                err
            ))
        })?;
        Self::init_schema(&conn)?;

        Ok(Self { db_path })
    }

    fn default_db_path() -> BabataResult<PathBuf> {
        Ok(babata_dir()?.join("task.db"))
    }

    fn connect(&self) -> BabataResult<Connection> {
        Connection::open(&self.db_path).map_err(|err| {
            BabataError::internal(format!(
                "Failed to open task db '{}': {}",
                self.db_path.display(),
                err
            ))
        })
    }

    fn init_schema(conn: &Connection) -> BabataResult<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                task_id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                parent_task_id TEXT,
                root_task_id TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                completed_at TEXT
            )",
            [],
        )
        .map_err(|err| BabataError::internal(format!("Failed to initialize tasks table: {}", err)))?;

        Ok(())
    }

    pub fn insert_task(
        &self,
        task_id: Uuid,
        parent_task_id: Option<Uuid>,
        root_task_id: Uuid,
    ) -> BabataResult<TaskRecord> {
        let status = TaskStatus::Running;
        let conn = self.connect()?;
        let parent_task_id_str = parent_task_id.map(|id| id.to_string());

        conn.execute(
            "INSERT INTO tasks (task_id, status, parent_task_id, root_task_id)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                task_id.to_string(),
                status.as_str(),
                parent_task_id_str,
                root_task_id.to_string()
            ],
        )
        .map_err(|err| {
            BabataError::internal(format!("Failed to insert task '{}' into sqlite: {}", task_id, err))
        })?;

        Ok(TaskRecord {
            task_id,
            status,
            parent_task_id,
            root_task_id,
        })
    }

    pub fn update_task_status(
        &self,
        task_id: Uuid,
        status: TaskStatus,
    ) -> BabataResult<()> {
        let conn = self.connect()?;
        let updated_rows = conn
            .execute(
                "UPDATE tasks
                 SET status = ?2,
                     updated_at = datetime('now'),
                     completed_at = CASE
                         WHEN ?2 = 'done' THEN COALESCE(completed_at, datetime('now'))
                         ELSE completed_at
                     END
                 WHERE task_id = ?1",
                params![task_id.to_string(), status.as_str()],
            )
            .map_err(|err| {
                BabataError::internal(format!(
                    "Failed to update task '{}' status in sqlite: {}",
                    task_id, err
                ))
            })?;

        if updated_rows == 0 {
            return Err(BabataError::internal(format!(
                "Task '{}' does not exist",
                task_id
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    fn temp_db_path() -> PathBuf {
        std::env::temp_dir()
            .join("babata-tests")
            .join(format!("task-store-{}.db", Uuid::new_v4()))
    }

    #[test]
    fn insert_task_persists_running_task() {
        let db_path = temp_db_path();
        let store = TaskStore::open(&db_path).expect("open task store");
        let task_id = Uuid::new_v4();
        let parent_task_id = Uuid::new_v4();
        let root_task_id = Uuid::new_v4();

        let record = store
            .insert_task(task_id, Some(parent_task_id), root_task_id)
            .expect("insert task");

        assert_eq!(
            record,
            TaskRecord {
                task_id,
                status: TaskStatus::Running,
                parent_task_id: Some(parent_task_id),
                root_task_id,
            }
        );

        let conn = Connection::open(&db_path).expect("open sqlite db");
        let row = conn
            .query_row(
                "SELECT status, parent_task_id, root_task_id, completed_at
                 FROM tasks
                 WHERE task_id = ?1",
                params![task_id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                    ))
                },
            )
            .expect("query inserted task");
        let expected_parent_task_id = parent_task_id.to_string();
        let expected_root_task_id = root_task_id.to_string();

        assert_eq!(row.0, "running");
        assert_eq!(row.1.as_deref(), Some(expected_parent_task_id.as_str()));
        assert_eq!(row.2.as_deref(), Some(expected_root_task_id.as_str()));
        assert!(row.3.is_none());

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn update_task_status_marks_task_done() {
        let db_path = temp_db_path();
        let store = TaskStore::open(&db_path).expect("open task store");
        let task_id = Uuid::new_v4();
        store
            .insert_task(task_id, None, task_id)
            .expect("insert task before update");

        store
            .update_task_status(task_id, TaskStatus::Done)
            .expect("update task status");

        let conn = Connection::open(&db_path).expect("open sqlite db");
        let row = conn
            .query_row(
                "SELECT status, completed_at
                 FROM tasks
                 WHERE task_id = ?1",
                params![task_id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                    ))
                },
            )
            .expect("query updated task");

        assert_eq!(row.0, "done");
        assert!(row.1.is_some());

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn update_task_status_returns_error_for_missing_task() {
        let db_path = temp_db_path();
        let store = TaskStore::open(&db_path).expect("open task store");
        let missing_task_id = Uuid::new_v4();

        let err = store
            .update_task_status(missing_task_id, TaskStatus::Paused)
            .expect_err("missing task should fail");

        assert!(err.to_string().contains("does not exist"));

        let _ = std::fs::remove_file(db_path);
    }
}
