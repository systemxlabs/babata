use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, Row, params, params_from_iter, types::Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    BabataResult,
    error::BabataError,
    task::{TaskListQuery, TaskStatus},
    utils::babata_dir,
};

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
                record.status.as_str(),
                record.parent_task_id.map(|id| id.to_string()),
                record.root_task_id.to_string(),
                record.created_at,
                record.never_ends,
            ],
        )
        .map_err(|err| BabataError::internal(format!("Failed to insert task row: {}", err)))?;
        Ok(())
    }

    pub fn update_task(
        &self,
        task_id: Uuid,
        description: Option<String>,
        never_ends: Option<bool>,
    ) -> BabataResult<()> {
        let conn = self.connect()?;
        let updated = match (description, never_ends) {
            (Some(description), Some(never_ends)) => conn
                .execute(
                    "UPDATE tasks SET description = ?1, never_ends = ?2 WHERE task_id = ?3",
                    params![description, never_ends, task_id.to_string()],
                )
                .map_err(|err| {
                    BabataError::internal(format!("Failed to update task row: {}", err))
                })?,
            (Some(description), None) => conn
                .execute(
                    "UPDATE tasks SET description = ?1 WHERE task_id = ?2",
                    params![description, task_id.to_string()],
                )
                .map_err(|err| {
                    BabataError::internal(format!("Failed to update task row: {}", err))
                })?,
            (None, Some(never_ends)) => conn
                .execute(
                    "UPDATE tasks SET never_ends = ?1 WHERE task_id = ?2",
                    params![never_ends, task_id.to_string()],
                )
                .map_err(|err| {
                    BabataError::internal(format!("Failed to update task row: {}", err))
                })?,
            (None, None) => {
                return Err(BabataError::internal(
                    "At least one task field must be provided for update".to_string(),
                ));
            }
        };

        if updated == 0 {
            return Err(BabataError::internal(format!(
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
            .ok_or_else(|| BabataError::internal(format!("Task '{}' not found", task_id)))?;

        Ok(task)
    }

    pub fn list_tasks(
        &self,
        status: Option<TaskStatus>,
        limit: usize,
        offset: Option<usize>,
    ) -> BabataResult<Vec<TaskRecord>> {
        self.list_tasks_filtered(&TaskListQuery {
            status,
            limit,
            offset,
            ..TaskListQuery::default()
        })
    }

    pub fn list_tasks_filtered(&self, query: &TaskListQuery) -> BabataResult<Vec<TaskRecord>> {
        if query.limit == 0 {
            return Ok(Vec::new());
        }

        let mut sql = String::from(
            "SELECT task_id, description, agent, status, parent_task_id, root_task_id, created_at, never_ends
             FROM tasks",
        );

        let mut clauses = Vec::new();
        let mut bind_params: Vec<Value> = Vec::new();

        if let Some(status) = query.status {
            clauses.push("status = ?".to_string());
            bind_params.push(Value::from(status.as_str().to_string()));
        }

        if query.root_only {
            clauses.push("parent_task_id IS NULL".to_string());
        }

        if let Some(root_task_id) = query.root_task_id {
            clauses.push("root_task_id = ?".to_string());
            bind_params.push(Value::from(root_task_id.to_string()));
        }

        if !clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&clauses.join(" AND "));
        }

        sql.push_str(" ORDER BY created_at DESC LIMIT ?");
        let limit = query.limit.min(i64::MAX as usize) as i64;
        bind_params.push(Value::from(limit));

        if let Some(offset) = query.offset {
            sql.push_str(" OFFSET ?");
            let offset = offset.min(i64::MAX as usize) as i64;
            bind_params.push(Value::from(offset));
        }

        let conn = self.connect()?;
        let mut stmt = conn.prepare(&sql).map_err(|err| {
            BabataError::internal(format!(
                "Failed to prepare task list query statement: {}",
                err
            ))
        })?;

        collect_task_records(
            stmt.query_map(params_from_iter(bind_params), parse_task_record)
                .map_err(|err| {
                    BabataError::internal(format!("Failed to query task rows: {}", err))
                })?,
        )
    }

    pub fn list_root_tree(&self, root_task_id: Uuid) -> BabataResult<Vec<TaskRecord>> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT task_id, description, agent, status, parent_task_id, root_task_id, created_at, never_ends
                 FROM tasks
                 WHERE root_task_id = ?1
                 ORDER BY created_at ASC",
            )
            .map_err(|err| {
                BabataError::internal(format!(
                    "Failed to prepare root tree query statement: {}",
                    err
                ))
            })?;

        collect_task_records(
            stmt.query_map(params![root_task_id.to_string()], parse_task_record)
                .map_err(|err| {
                    BabataError::internal(format!("Failed to query root tree rows: {}", err))
                })?,
        )
    }

    pub fn count_tasks(&self, status: Option<TaskStatus>) -> BabataResult<usize> {
        let conn = self.connect()?;
        let count = match status {
            Some(status) => conn
                .query_row(
                    "SELECT COUNT(*) FROM tasks WHERE status = ?1",
                    params![status.as_str()],
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
    use crate::task::TaskListQuery;

    fn temp_db_path(test_name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("babata-{test_name}-{}.db", Uuid::new_v4()))
    }

    #[test]
    fn task_record_requires_never_ends_when_deserializing() {
        let task_id = Uuid::new_v4();
        let mut payload = serde_json::to_value(TaskRecord {
            task_id,
            description: "missing never_ends".to_string(),
            agent: Some("codex".to_string()),
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
                agent: Some("codex".to_string()),
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
    fn update_task_persists_description_and_never_ends() {
        let db_path = temp_db_path("task-store-update-task");
        let store = TaskStore::open(&db_path).expect("open store");
        let task_id = Uuid::new_v4();

        store
            .insert_task(TaskRecord {
                task_id,
                description: "before".to_string(),
                agent: Some("codex".to_string()),
                status: TaskStatus::Running,
                parent_task_id: None,
                root_task_id: task_id,
                created_at: 789,
                never_ends: false,
            })
            .expect("insert task");

        store
            .update_task(task_id, Some("after".to_string()), Some(true))
            .expect("update task");

        let task = store.get_task(task_id).expect("load task");
        assert_eq!(task.description, "after");
        assert!(task.never_ends);

        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn list_tasks_filters_root_only() {
        let db_path = temp_db_path("task-store-root-only");
        let store = TaskStore::open(&db_path).expect("open store");

        let root_a = Uuid::new_v4();
        let sub_a = Uuid::new_v4();
        let root_b = Uuid::new_v4();

        store
            .insert_task(TaskRecord {
                task_id: root_a,
                description: "root a".to_string(),
                agent: None,
                status: TaskStatus::Running,
                parent_task_id: None,
                root_task_id: root_a,
                created_at: 100,
                never_ends: false,
            })
            .expect("insert root a");

        store
            .insert_task(TaskRecord {
                task_id: sub_a,
                description: "sub a".to_string(),
                agent: None,
                status: TaskStatus::Running,
                parent_task_id: Some(root_a),
                root_task_id: root_a,
                created_at: 200,
                never_ends: false,
            })
            .expect("insert sub a");

        store
            .insert_task(TaskRecord {
                task_id: root_b,
                description: "root b".to_string(),
                agent: None,
                status: TaskStatus::Running,
                parent_task_id: None,
                root_task_id: root_b,
                created_at: 300,
                never_ends: false,
            })
            .expect("insert root b");

        let tasks = store
            .list_tasks_filtered(&TaskListQuery {
                root_only: true,
                ..TaskListQuery::default()
            })
            .expect("list tasks");

        assert_eq!(tasks.len(), 2);
        assert!(tasks.iter().all(|task| task.parent_task_id.is_none()));
        assert_eq!(tasks[0].task_id, root_b);
        assert_eq!(tasks[1].task_id, root_a);

        let _ = std::fs::remove_file(&db_path);
    }
}
