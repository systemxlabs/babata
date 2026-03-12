use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};
use uuid::Uuid;

use crate::{BabataResult, error::BabataError, task::TaskStatus, utils::babata_dir};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRecord {
    pub task_id: Uuid,
    pub status: TaskStatus,
    pub parent_task_id: Option<Uuid>,
    pub root_task_id: Uuid,
    pub created_at: i64,
}

#[derive(Debug)]
pub struct TaskStore {
    db_path: PathBuf,
}

impl TaskStore {
    pub fn new() -> BabataResult<Self> {
        let db_path = Self::default_db_path()?;
        Self::open(db_path)
    }

    pub fn insert_task(&self, record: TaskRecord) -> BabataResult<()> {
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO tasks (task_id, status, parent_task_id, root_task_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                record.task_id.to_string(),
                record.status.as_str(),
                record.parent_task_id.map(|id| id.to_string()),
                record.root_task_id.to_string(),
                record.created_at,
            ],
        )
        .map_err(|err| BabataError::internal(format!("Failed to insert task row: {}", err)))?;
        Ok(())
    }

    pub fn update_task_status(&self, task_id: Uuid, status: TaskStatus) -> BabataResult<()> {
        let conn = self.connect()?;
        let updated = conn
            .execute(
                "UPDATE tasks SET status = ?1 WHERE task_id = ?2",
                params![status.as_str(), task_id.to_string()],
            )
            .map_err(|err| {
                BabataError::internal(format!("Failed to update task status row: {}", err))
            })?;

        if updated == 0 {
            return Err(BabataError::internal(format!(
                "Task '{}' not found",
                task_id
            )));
        }

        Ok(())
    }

    pub fn get_task(&self, task_id: Uuid) -> BabataResult<TaskRecord> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT task_id, status, parent_task_id, root_task_id, created_at
                 FROM tasks
                 WHERE task_id = ?1",
            )
            .map_err(|err| {
                BabataError::internal(format!("Failed to prepare task query statement: {}", err))
            })?;

        let row = stmt
            .query_row(params![task_id.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })
            .optional()
            .map_err(|err| BabataError::internal(format!("Failed to query task row: {}", err)))?
            .ok_or_else(|| BabataError::internal(format!("Task '{}' not found", task_id)))?;

        let parsed_task_id = Uuid::parse_str(&row.0).map_err(|err| {
            BabataError::internal(format!(
                "Failed to parse task_id '{}' as UUID: {}",
                row.0, err
            ))
        })?;
        let status = row.1.parse::<TaskStatus>().map_err(|err| {
            BabataError::internal(format!("Failed to parse task status '{}': {}", row.1, err))
        })?;
        let parent_task_id = row
            .2
            .map(|value| {
                Uuid::parse_str(&value).map_err(|err| {
                    BabataError::internal(format!(
                        "Failed to parse parent_task_id '{}' as UUID: {}",
                        value, err
                    ))
                })
            })
            .transpose()?;
        let root_task_id = Uuid::parse_str(&row.3).map_err(|err| {
            BabataError::internal(format!(
                "Failed to parse root_task_id '{}' as UUID: {}",
                row.3, err
            ))
        })?;

        Ok(TaskRecord {
            task_id: parsed_task_id,
            status,
            parent_task_id,
            root_task_id,
            created_at: row.4,
        })
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
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                task_id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                parent_task_id TEXT,
                root_task_id TEXT NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|err| {
            BabataError::internal(format!("Failed to initialize tasks table: {}", err))
        })?;

        Ok(Self { db_path })
    }

    fn default_db_path() -> BabataResult<PathBuf> {
        Ok(babata_dir()?.join("task").join("task.db"))
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
}
