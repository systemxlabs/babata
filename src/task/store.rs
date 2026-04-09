use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, Row, params};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{BabataResult, error::BabataError, task::TaskStatus, utils::babata_dir};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskRecord {
    pub task_id: Uuid,
    pub description: String,
    pub agent: Option<String>,
    pub status: TaskStatus,
    pub parent_task_id: Option<Uuid>,
    pub root_task_id: Uuid,
    pub created_at: i64,
    pub never_ends: bool,
}

#[derive(Debug, Clone)]
pub struct TaskStore {
    db_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(tag = "field", rename_all = "snake_case", deny_unknown_fields)]
pub enum TaskUpdate {
    Description {
        #[schemars(description = "New task description", length(min = 1))]
        description: String,
    },
    NeverEnds {
        #[schemars(description = "New value for the task's never_ends flag")]
        never_ends: bool,
    },
}

impl TaskStore {
    pub fn new() -> BabataResult<Self> {
        let db_path = Self::default_db_path()?;
        Self::open(db_path)
    }

    pub fn insert_task(&self, record: TaskRecord) -> BabataResult<()> {
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO tasks (task_id, description, agent, status, parent_task_id, root_task_id, created_at, never_ends)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                record.task_id.to_string(),
                record.description,
                record.agent,
                record.status.to_string(),
                record.parent_task_id.map(|id| id.to_string()),
                record.root_task_id.to_string(),
                record.created_at,
                record.never_ends,
            ],
        )
        .map_err(|err| BabataError::internal(format!("Failed to insert task row: {}", err)))?;
        Ok(())
    }

    pub fn update_task(&self, task_id: Uuid, update: TaskUpdate) -> BabataResult<()> {
        let conn = self.connect()?;
        let updated = match update {
            TaskUpdate::Description { description } => conn
                .execute(
                    "UPDATE tasks SET description = ?1 WHERE task_id = ?2",
                    params![description, task_id.to_string()],
                )
                .map_err(|err| {
                    BabataError::internal(format!("Failed to update task row: {}", err))
                })?,
            TaskUpdate::NeverEnds { never_ends } => conn
                .execute(
                    "UPDATE tasks SET never_ends = ?1 WHERE task_id = ?2",
                    params![never_ends, task_id.to_string()],
                )
                .map_err(|err| {
                    BabataError::internal(format!("Failed to update task row: {}", err))
                })?,
        };

        if updated == 0 {
            return Err(BabataError::not_found(format!(
                "Task '{}' not found",
                task_id
            )));
        }

        Ok(())
    }

    pub fn update_task_status(&self, task_id: Uuid, status: TaskStatus) -> BabataResult<()> {
        let conn = self.connect()?;
        let updated = conn
            .execute(
                "UPDATE tasks SET status = ?1 WHERE task_id = ?2",
                params![status.to_string(), task_id.to_string()],
            )
            .map_err(|err| {
                BabataError::internal(format!("Failed to update task status row: {}", err))
            })?;

        if updated == 0 {
            return Err(BabataError::not_found(format!(
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
                "SELECT task_id, description, agent, status, parent_task_id, root_task_id, created_at, never_ends
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
            .ok_or_else(|| BabataError::not_found(format!("Task '{}' not found", task_id)))?;

        Ok(task)
    }

    pub fn task_exists(&self, task_id: Uuid) -> BabataResult<bool> {
        let conn = self.connect()?;
        conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE task_id = ?1)",
            params![task_id.to_string()],
            |row| row.get::<_, bool>(0),
        )
        .map_err(|err| BabataError::internal(format!("Failed to query task existence: {}", err)))
    }

    pub fn list_tasks(
        &self,
        status: Option<TaskStatus>,
        limit: usize,
        offset: Option<usize>,
    ) -> BabataResult<Vec<TaskRecord>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let conn = self.connect()?;
        let tasks = match (status, offset) {
            (Some(status), Some(offset)) => {
                let mut stmt = conn
                    .prepare(
                        "SELECT task_id, description, agent, status, parent_task_id, root_task_id, created_at, never_ends
                         FROM tasks
                         WHERE status = ?1
                         ORDER BY created_at DESC
                         LIMIT ?2 OFFSET ?3",
                    )
                    .map_err(|err| {
                        BabataError::internal(format!(
                            "Failed to prepare task list query statement: {}",
                            err
                        ))
                    })?;
                collect_task_records(
                    stmt.query_map(
                        params![status.to_string(), limit as i64, offset as i64],
                        parse_task_record,
                    )
                    .map_err(|err| {
                        BabataError::internal(format!("Failed to query task rows: {}", err))
                    })?,
                )?
            }
            (Some(status), None) => {
                let mut stmt = conn
                    .prepare(
                        "SELECT task_id, description, agent, status, parent_task_id, root_task_id, created_at, never_ends
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
                    stmt.query_map(params![status.to_string(), limit as i64], parse_task_record)
                        .map_err(|err| {
                            BabataError::internal(format!("Failed to query task rows: {}", err))
                        })?,
                )?
            }
            (None, Some(offset)) => {
                let mut stmt = conn
                    .prepare(
                        "SELECT task_id, description, agent, status, parent_task_id, root_task_id, created_at, never_ends
                         FROM tasks
                         ORDER BY created_at DESC
                         LIMIT ?1 OFFSET ?2",
                    )
                    .map_err(|err| {
                        BabataError::internal(format!(
                            "Failed to prepare task list query statement: {}",
                            err
                        ))
                    })?;
                collect_task_records(
                    stmt.query_map(params![limit as i64, offset as i64], parse_task_record)
                        .map_err(|err| {
                            BabataError::internal(format!("Failed to query task rows: {}", err))
                        })?,
                )?
            }
            (None, None) => {
                let mut stmt = conn
                    .prepare(
                        "SELECT task_id, description, agent, status, parent_task_id, root_task_id, created_at, never_ends
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
        };

        Ok(tasks)
    }

    pub fn count_tasks(&self, status: Option<TaskStatus>) -> BabataResult<usize> {
        let conn = self.connect()?;
        let count = match status {
            Some(status) => conn
                .query_row(
                    "SELECT COUNT(*) FROM tasks WHERE status = ?1",
                    params![status.to_string()],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|err| {
                    BabataError::internal(format!("Failed to count filtered task rows: {}", err))
                })?,
            None => conn
                .query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get::<_, i64>(0))
                .map_err(|err| {
                    BabataError::internal(format!("Failed to count task rows: {}", err))
                })?,
        };

        usize::try_from(count).map_err(|err| {
            BabataError::internal(format!(
                "Failed to convert task count '{}' to usize: {}",
                count, err
            ))
        })
    }

    /// Execute a raw SQL query and return results as JSON array
    /// Note: This is intended for SELECT queries only. Results are returned as
    /// an array of JSON objects where keys are column names and values are the data.
    pub fn query_sql(
        &self,
        sql: &str,
    ) -> BabataResult<Vec<serde_json::Map<String, serde_json::Value>>> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(sql)
            .map_err(|err| BabataError::tool(format!("Failed to prepare SQL query: {}", err)))?;

        let column_names: Vec<String> = stmt
            .column_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        let rows = stmt
            .query_map([], |row| {
                let mut obj = serde_json::Map::new();
                for (idx, col_name) in column_names.iter().enumerate() {
                    let value = match row.get_ref(idx) {
                        Ok(rusqlite::types::ValueRef::Null) => serde_json::Value::Null,
                        Ok(rusqlite::types::ValueRef::Integer(i)) => {
                            serde_json::Value::Number(i.into())
                        }
                        Ok(rusqlite::types::ValueRef::Real(f)) => serde_json::Number::from_f64(f)
                            .map_or(serde_json::Value::Null, serde_json::Value::Number),
                        Ok(rusqlite::types::ValueRef::Text(s)) => {
                            serde_json::Value::String(String::from_utf8_lossy(s).to_string())
                        }
                        Ok(rusqlite::types::ValueRef::Blob(_)) => {
                            serde_json::Value::String("<blob>".to_string())
                        }
                        Err(_) => serde_json::Value::Null,
                    };
                    obj.insert(col_name.clone(), value);
                }
                Ok(obj)
            })
            .map_err(|err| BabataError::tool(format!("Failed to execute SQL query: {}", err)))?;

        rows.map(|row| row.map_err(|err| BabataError::tool(format!("Failed to read row: {}", err))))
            .collect()
    }

    pub fn list_subtasks(&self, parent_task_id: Uuid) -> BabataResult<Vec<TaskRecord>> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT task_id, description, agent, status, parent_task_id, root_task_id, created_at, never_ends
                 FROM tasks
                 WHERE parent_task_id = ?1
                 ORDER BY created_at DESC",
            )
            .map_err(|err| {
                BabataError::internal(format!(
                    "Failed to prepare subtask list query statement: {}",
                    err
                ))
            })?;

        collect_task_records(
            stmt.query_map(params![parent_task_id.to_string()], parse_task_record)
                .map_err(|err| {
                    BabataError::internal(format!("Failed to query subtask rows: {}", err))
                })?,
        )
    }

    pub fn list_all_subtasks(&self, task_id: Uuid) -> BabataResult<Vec<TaskRecord>> {
        let mut all_subtasks = Vec::new();
        let mut queue = vec![task_id];

        while let Some(current_id) = queue.pop() {
            let subtasks = self.list_subtasks(current_id)?;
            for subtask in subtasks {
                queue.push(subtask.task_id);
                all_subtasks.push(subtask);
            }
        }

        Ok(all_subtasks)
    }

    pub fn delete_task(&self, task_id: Uuid) -> BabataResult<()> {
        let conn = self.connect()?;
        let deleted = conn
            .execute(
                "DELETE FROM tasks WHERE task_id = ?1",
                params![task_id.to_string()],
            )
            .map_err(|err| BabataError::internal(format!("Failed to delete task row: {}", err)))?;

        if deleted == 0 {
            return Err(BabataError::not_found(format!(
                "Task '{}' not found",
                task_id
            )));
        }

        Ok(())
    }

    pub(crate) fn open(db_path: impl AsRef<Path>) -> BabataResult<Self> {
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
                description TEXT NOT NULL,
                agent TEXT,
                status TEXT NOT NULL,
                parent_task_id TEXT,
                root_task_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                never_ends INTEGER NOT NULL DEFAULT 0
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
    let description = row.get::<_, String>(1)?;
    let agent = row.get::<_, Option<String>>(2)?;
    let status_raw = row.get::<_, String>(3)?;
    let parent_task_id_raw = row.get::<_, Option<String>>(4)?;
    let root_task_id_raw = row.get::<_, String>(5)?;
    let created_at = row.get::<_, i64>(6)?;
    let never_ends = row.get::<_, bool>(7)?;

    let task_id = Uuid::parse_str(&task_id_raw).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
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
        description,
        agent,
        status,
        parent_task_id,
        root_task_id,
        created_at,
        never_ends,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db_path(test_name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("babata-{test_name}-{}.db", Uuid::new_v4()))
    }

    #[test]
    fn task_record_requires_never_ends_when_deserializing() {
        let task_id = Uuid::new_v4();
        let mut payload = serde_json::to_value(TaskRecord {
            task_id,
            description: "missing never_ends".to_string(),
            agent: Some("babata".to_string()),
            status: TaskStatus::Running,
            parent_task_id: None,
            root_task_id: task_id,
            created_at: 123,
            never_ends: true,
        })
        .expect("serialize task record");
        payload
            .as_object_mut()
            .expect("task record json object")
            .remove("never_ends");

        let error = serde_json::from_value::<TaskRecord>(payload)
            .expect_err("missing never_ends should fail");

        assert!(error.to_string().contains("never_ends"));
    }

    #[test]
    fn insert_task_persists_never_ends() {
        let db_path = temp_db_path("task-store-never-ends");
        let store = TaskStore::open(&db_path).expect("open store");
        let task_id = Uuid::new_v4();

        store
            .insert_task(TaskRecord {
                task_id,
                description: "persist never_ends".to_string(),
                agent: Some("babata".to_string()),
                status: TaskStatus::Running,
                parent_task_id: None,
                root_task_id: task_id,
                created_at: 456,
                never_ends: true,
            })
            .expect("insert task");

        let task = store.get_task(task_id).expect("load task");
        assert!(task.never_ends);

        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn update_task_persists_single_field_updates() {
        let db_path = temp_db_path("task-store-update-task");
        let store = TaskStore::open(&db_path).expect("open store");
        let task_id = Uuid::new_v4();

        store
            .insert_task(TaskRecord {
                task_id,
                description: "before".to_string(),
                agent: Some("babata".to_string()),
                status: TaskStatus::Running,
                parent_task_id: None,
                root_task_id: task_id,
                created_at: 789,
                never_ends: false,
            })
            .expect("insert task");

        store
            .update_task(
                task_id,
                TaskUpdate::Description {
                    description: "after".to_string(),
                },
            )
            .expect("update description");
        store
            .update_task(task_id, TaskUpdate::NeverEnds { never_ends: true })
            .expect("update never_ends");

        let task = store.get_task(task_id).expect("load task");
        assert_eq!(task.description, "after");
        assert!(task.never_ends);

        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn task_exists_returns_true_only_for_inserted_task() {
        let db_path = temp_db_path("task-store-exists");
        let store = TaskStore::open(&db_path).expect("open store");
        let task_id = Uuid::new_v4();

        store
            .insert_task(TaskRecord {
                task_id,
                description: "exists".to_string(),
                agent: Some("babata".to_string()),
                status: TaskStatus::Running,
                parent_task_id: None,
                root_task_id: task_id,
                created_at: 999,
                never_ends: false,
            })
            .expect("insert task");

        assert!(store.task_exists(task_id).expect("existing task"));
        assert!(
            !store
                .task_exists(Uuid::new_v4())
                .expect("missing task should be false")
        );

        let _ = std::fs::remove_file(&db_path);
    }
}
