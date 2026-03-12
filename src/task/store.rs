use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, Row, params};
use uuid::Uuid;

use crate::{
    BabataResult, error::BabataError, message::Content, task::TaskStatus, utils::babata_dir,
};

#[derive(Debug, Clone, PartialEq)]
pub struct TaskRecord {
    pub task_id: Uuid,
    pub prompt: Vec<Content>,
    pub agent: Option<String>,
    pub status: TaskStatus,
    pub parent_task_id: Option<Uuid>,
    pub root_task_id: Uuid,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
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
        let prompt_json = serde_json::to_string(&record.prompt).map_err(|err| {
            BabataError::internal(format!(
                "Failed to serialize task prompt into JSON: {}",
                err
            ))
        })?;
        conn.execute(
            "INSERT INTO tasks (task_id, prompt, agent, status, parent_task_id, root_task_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                record.task_id.to_string(),
                prompt_json,
                record.agent,
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
                "SELECT task_id, prompt, agent, status, parent_task_id, root_task_id, created_at
                 FROM tasks
                 WHERE task_id = ?1",
            )
            .map_err(|err| {
                BabataError::internal(format!("Failed to prepare task query statement: {}", err))
            })?;

        let task = stmt
            .query_row(params![task_id.to_string()], parse_task_record)
            .optional()
            .map_err(|err| BabataError::internal(format!("Failed to query task row: {}", err)))?
            .ok_or_else(|| BabataError::internal(format!("Task '{}' not found", task_id)))?;

        Ok(task)
    }

    pub fn list_tasks(
        &self,
        status: Option<TaskStatus>,
        limit: Option<usize>,
    ) -> BabataResult<Vec<TaskRecord>> {
        if matches!(limit, Some(0)) {
            return Ok(Vec::new());
        }

        let conn = self.connect()?;
        let tasks = match (status, limit) {
            (Some(status), Some(limit)) => {
                let mut stmt = conn
                    .prepare(
                        "SELECT task_id, prompt, agent, status, parent_task_id, root_task_id, created_at
                         FROM tasks
                         WHERE status = ?1
                         ORDER BY created_at DESC
                         LIMIT ?2",
                    )
                    .map_err(|err| {
                        BabataError::internal(format!(
                            "Failed to prepare task list query statement: {}",
                            err
                        ))
                    })?;
                collect_task_records(
                    stmt.query_map(params![status.as_str(), limit as i64], parse_task_record)
                        .map_err(|err| {
                            BabataError::internal(format!("Failed to query task rows: {}", err))
                        })?,
                )?
            }
            (Some(status), None) => {
                let mut stmt = conn
                    .prepare(
                        "SELECT task_id, prompt, agent, status, parent_task_id, root_task_id, created_at
                         FROM tasks
                         WHERE status = ?1
                         ORDER BY created_at DESC",
                    )
                    .map_err(|err| {
                        BabataError::internal(format!(
                            "Failed to prepare task list query statement: {}",
                            err
                        ))
                    })?;
                collect_task_records(
                    stmt.query_map(params![status.as_str()], parse_task_record)
                        .map_err(|err| {
                            BabataError::internal(format!("Failed to query task rows: {}", err))
                        })?,
                )?
            }
            (None, Some(limit)) => {
                let mut stmt = conn
                    .prepare(
                        "SELECT task_id, prompt, agent, status, parent_task_id, root_task_id, created_at
                         FROM tasks
                         ORDER BY created_at DESC
                         LIMIT ?1",
                    )
                    .map_err(|err| {
                        BabataError::internal(format!(
                            "Failed to prepare task list query statement: {}",
                            err
                        ))
                    })?;
                collect_task_records(
                    stmt.query_map(params![limit as i64], parse_task_record)
                        .map_err(|err| {
                            BabataError::internal(format!("Failed to query task rows: {}", err))
                        })?,
                )?
            }
            (None, None) => {
                let mut stmt = conn
                    .prepare(
                        "SELECT task_id, prompt, agent, status, parent_task_id, root_task_id, created_at
                         FROM tasks
                         ORDER BY created_at DESC",
                    )
                    .map_err(|err| {
                        BabataError::internal(format!(
                            "Failed to prepare task list query statement: {}",
                            err
                        ))
                    })?;
                collect_task_records(stmt.query_map([], parse_task_record).map_err(|err| {
                    BabataError::internal(format!("Failed to query task rows: {}", err))
                })?)?
            }
        };

        Ok(tasks)
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
                prompt TEXT NOT NULL,
                agent TEXT,
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
}

fn parse_task_record(row: &Row<'_>) -> rusqlite::Result<TaskRecord> {
    let task_id_raw = row.get::<_, String>(0)?;
    let prompt_raw = row.get::<_, String>(1)?;
    let agent = row.get::<_, Option<String>>(2)?;
    let status_raw = row.get::<_, String>(3)?;
    let parent_task_id_raw = row.get::<_, Option<String>>(4)?;
    let root_task_id_raw = row.get::<_, String>(5)?;
    let created_at = row.get::<_, i64>(6)?;

    let task_id = Uuid::parse_str(&task_id_raw).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
    })?;
    let prompt = serde_json::from_str::<Vec<Content>>(&prompt_raw).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(err))
    })?;
    let status = status_raw.parse::<TaskStatus>().map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(
            3,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
        )
    })?;
    let parent_task_id = parent_task_id_raw
        .map(|value| {
            Uuid::parse_str(&value).map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(
                    4,
                    rusqlite::types::Type::Text,
                    Box::new(err),
                )
            })
        })
        .transpose()?;
    let root_task_id = Uuid::parse_str(&root_task_id_raw).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(err))
    })?;

    Ok(TaskRecord {
        task_id,
        prompt,
        agent,
        status,
        parent_task_id,
        root_task_id,
        created_at,
    })
}

fn collect_task_records(
    rows: impl Iterator<Item = rusqlite::Result<TaskRecord>>,
) -> BabataResult<Vec<TaskRecord>> {
    rows.map(|row| {
        row.map_err(|err| BabataError::internal(format!("Failed to decode task row: {}", err)))
    })
    .collect()
}
