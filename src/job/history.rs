use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::{BabataResult, error::BabataError, utils::babata_dir};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobRunStatus {
    Success,
    Failed,
}

impl JobRunStatus {
    fn as_str(self) -> &'static str {
        match self {
            JobRunStatus::Success => "success",
            JobRunStatus::Failed => "failed",
        }
    }

    fn from_str(value: &str) -> BabataResult<Self> {
        match value {
            "success" => Ok(JobRunStatus::Success),
            "failed" => Ok(JobRunStatus::Failed),
            _ => Err(BabataError::memory(format!(
                "Invalid job run status '{}' in sqlite row",
                value
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JobHistoryEntry {
    pub job_name: String,
    pub job_config: String,
    pub status: JobRunStatus,
    pub response: Option<String>,
    pub error: Option<String>,
    pub started_at_epoch: i64,
    pub finished_at_epoch: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone)]
pub struct JobHistoryStore {
    db_path: PathBuf,
}

type RawJobHistoryRow = (
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    i64,
    i64,
    i64,
);

impl JobHistoryStore {
    pub fn new() -> BabataResult<Self> {
        let db_path = Self::default_db_path()?;
        Self::open(db_path)
    }

    fn default_db_path() -> BabataResult<PathBuf> {
        let dir = babata_dir()?;
        Ok(dir.join("job_history.db"))
    }

    fn open(path: impl AsRef<Path>) -> BabataResult<Self> {
        let path = path.as_ref();
        let Some(parent) = path.parent() else {
            return Err(BabataError::memory(format!(
                "Invalid sqlite path '{}'",
                path.display()
            )));
        };

        std::fs::create_dir_all(parent).map_err(|err| {
            BabataError::memory(format!(
                "Failed to create job history db directory '{}': {}",
                parent.display(),
                err
            ))
        })?;

        let store = Self {
            db_path: path.to_path_buf(),
        };
        store.init_table()?;
        Ok(store)
    }

    fn connect(&self) -> BabataResult<Connection> {
        Connection::open(&self.db_path).map_err(|err| {
            BabataError::memory(format!(
                "Failed to open job history db '{}': {}",
                self.db_path.display(),
                err
            ))
        })
    }

    fn init_table(&self) -> BabataResult<()> {
        let conn = self.connect()?;
        conn.execute(create_table_sql(), []).map_err(|err| {
            BabataError::memory(format!("Failed to initialize job_history table: {}", err))
        })?;

        Ok(())
    }

    pub fn insert(&self, entry: &JobHistoryEntry) -> BabataResult<()> {
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO job_history (
                job_name,
                job_config,
                status,
                response,
                error,
                started_at_epoch,
                finished_at_epoch,
                duration_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                entry.job_name,
                entry.job_config,
                entry.status.as_str(),
                entry.response,
                entry.error,
                entry.started_at_epoch,
                entry.finished_at_epoch,
                entry.duration_ms,
            ],
        )
        .map_err(|err| BabataError::memory(format!("Failed to insert job history row: {}", err)))?;
        Ok(())
    }

    pub fn query(
        &self,
        job_name: Option<&str>,
        limit: usize,
    ) -> BabataResult<Vec<JobHistoryEntry>> {
        if limit == 0 {
            return Err(BabataError::config(
                "Job history query limit must be greater than 0",
            ));
        }
        let limit = limit.min(i64::MAX as usize) as i64;
        let conn = self.connect()?;

        let mut rows = Vec::new();
        if let Some(job_name) = job_name {
            let mut stmt = conn
                .prepare(
                    "SELECT
                        job_name,
                        job_config,
                        status,
                        response,
                        error,
                        started_at_epoch,
                        finished_at_epoch,
                        duration_ms
                    FROM job_history
                    WHERE job_name = ?1
                    ORDER BY started_at_epoch DESC, finished_at_epoch DESC, rowid DESC
                    LIMIT ?2",
                )
                .map_err(|err| {
                    BabataError::memory(format!(
                        "Failed to prepare job history query statement: {}",
                        err
                    ))
                })?;
            let mapped = stmt
                .query_map(params![job_name, limit], map_history_row)
                .map_err(|err| {
                    BabataError::memory(format!("Failed to query job history rows: {}", err))
                })?;
            for row in mapped {
                let raw = row.map_err(|err| {
                    BabataError::memory(format!("Failed to scan job history sqlite row: {}", err))
                })?;
                rows.push(build_history_entry(raw)?);
            }
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT
                        job_name,
                        job_config,
                        status,
                        response,
                        error,
                        started_at_epoch,
                        finished_at_epoch,
                        duration_ms
                    FROM job_history
                    ORDER BY started_at_epoch DESC, finished_at_epoch DESC, rowid DESC
                    LIMIT ?1",
                )
                .map_err(|err| {
                    BabataError::memory(format!(
                        "Failed to prepare job history query statement: {}",
                        err
                    ))
                })?;
            let mapped = stmt
                .query_map(params![limit], map_history_row)
                .map_err(|err| {
                    BabataError::memory(format!("Failed to query job history rows: {}", err))
                })?;
            for row in mapped {
                let raw = row.map_err(|err| {
                    BabataError::memory(format!("Failed to scan job history sqlite row: {}", err))
                })?;
                rows.push(build_history_entry(raw)?);
            }
        }

        Ok(rows)
    }
}

fn create_table_sql() -> &'static str {
    "CREATE TABLE IF NOT EXISTS job_history (
        job_name TEXT NOT NULL,
        job_config TEXT NOT NULL,
        status TEXT NOT NULL,
        response TEXT,
        error TEXT,
        started_at_epoch INTEGER NOT NULL,
        finished_at_epoch INTEGER NOT NULL,
        duration_ms INTEGER NOT NULL,
        created_at TEXT NOT NULL DEFAULT (datetime('now'))
    )"
}

fn map_history_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawJobHistoryRow> {
    Ok((
        row.get::<_, String>(0)?,
        row.get::<_, String>(1)?,
        row.get::<_, String>(2)?,
        row.get::<_, Option<String>>(3)?,
        row.get::<_, Option<String>>(4)?,
        row.get::<_, i64>(5)?,
        row.get::<_, i64>(6)?,
        row.get::<_, i64>(7)?,
    ))
}

fn build_history_entry(raw: RawJobHistoryRow) -> BabataResult<JobHistoryEntry> {
    let (
        job_name,
        job_config,
        status,
        response,
        error,
        started_at_epoch,
        finished_at_epoch,
        duration_ms,
    ) = raw;
    Ok(JobHistoryEntry {
        job_name,
        job_config,
        status: JobRunStatus::from_str(&status)?,
        response,
        error,
        started_at_epoch,
        finished_at_epoch,
        duration_ms,
    })
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    #[test]
    fn insert_and_query_history_roundtrip() {
        let db_path = std::env::temp_dir()
            .join("babata-tests")
            .join(format!("job-history-store-{}.db", Uuid::new_v4()));

        let store = JobHistoryStore::open(&db_path).expect("open sqlite job history store");
        store
            .insert(&JobHistoryEntry {
                job_name: "daily".to_string(),
                job_config: "{\"name\":\"daily\"}".to_string(),
                status: JobRunStatus::Success,
                response: Some("{\"type\":\"assistant_response\"}".to_string()),
                error: None,
                started_at_epoch: 1700000000,
                finished_at_epoch: 1700000002,
                duration_ms: 2345,
            })
            .expect("insert first row");
        store
            .insert(&JobHistoryEntry {
                job_name: "weekly".to_string(),
                job_config: "{\"name\":\"weekly\"}".to_string(),
                status: JobRunStatus::Failed,
                response: None,
                error: Some("provider timeout".to_string()),
                started_at_epoch: 1700000100,
                finished_at_epoch: 1700000105,
                duration_ms: 5000,
            })
            .expect("insert second row");

        let all_rows = store.query(None, 10).expect("query all rows");
        assert_eq!(all_rows.len(), 2);
        assert_eq!(all_rows[0].job_name, "weekly");
        assert_eq!(all_rows[0].job_config, "{\"name\":\"weekly\"}");
        assert_eq!(all_rows[1].job_name, "daily");

        let filtered = store.query(Some("daily"), 10).expect("query filtered rows");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].job_name, "daily");
        assert_eq!(filtered[0].status, JobRunStatus::Success);

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn query_respects_limit() {
        let db_path = std::env::temp_dir()
            .join("babata-tests")
            .join(format!("job-history-store-{}.db", Uuid::new_v4()));
        let store = JobHistoryStore::open(&db_path).expect("open sqlite job history store");

        for idx in 0..3 {
            store
                .insert(&JobHistoryEntry {
                    job_name: format!("job-{idx}"),
                    job_config: format!("{{\"name\":\"job-{idx}\"}}"),
                    status: JobRunStatus::Success,
                    response: Some("ok".to_string()),
                    error: None,
                    started_at_epoch: 1000 + idx,
                    finished_at_epoch: 1001 + idx,
                    duration_ms: 1,
                })
                .expect("insert history row");
        }

        let rows = store.query(None, 2).expect("query limited rows");
        assert_eq!(rows.len(), 2);

        let _ = std::fs::remove_file(db_path);
    }
}
