use std::path::PathBuf;

use crate::BabataResult;
use crate::error::BabataError;
use crate::utils::babata_dir;
use rusqlite::ffi::sqlite3_auto_extension;
use rusqlite::{Connection, Result, params};

pub struct MemoryStore {
    conn: Connection,
    dimension: usize,
}

impl MemoryStore {
    pub fn new(db_path: PathBuf, dimension: usize) -> BabataResult<Self> {
        // Enable libsimple for Chinese FTS5
        libsimple::enable_auto_extension()
            .map_err(|e| BabataError::memory(format!("Failed to enable libsimple: {}", e)))?;

        // Release jieba dictionary files
        let jieba_dir = Self::jieba_dict_dir()?;
        libsimple::release_jieba_dict(&jieba_dir)
            .map_err(|e| BabataError::memory(format!("Failed to release jieba dict: {}", e)))?;

        // Enable sqlite-vec for vector search
        unsafe {
            #[allow(clippy::missing_transmute_annotations)]
            sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite_vec::sqlite3_vec_init as *const (),
            )));
        }

        Self::init_schema(&db_path, dimension, &jieba_dir)
    }

    pub fn default_db_path() -> BabataResult<PathBuf> {
        let dir = babata_dir()?;
        let path = dir.join("message.db");

        let Some(parent) = path.parent() else {
            return Err(BabataError::memory(format!(
                "Invalid sqlite path '{}'",
                path.display()
            )));
        };

        std::fs::create_dir_all(parent).map_err(|err| {
            BabataError::memory(format!(
                "Failed to create message db directory '{}': {}",
                parent.display(),
                err
            ))
        })?;

        Ok(path)
    }

    fn jieba_dict_dir() -> BabataResult<PathBuf> {
        let dir = babata_dir()?;
        let path = dir.join("jieba_dict");

        std::fs::create_dir_all(&path).map_err(|err| {
            BabataError::memory(format!(
                "Failed to create jieba dict directory '{}': {}",
                path.display(),
                err
            ))
        })?;

        Ok(path)
    }

    fn init_schema(path: &PathBuf, dimension: usize, jieba_dir: &PathBuf) -> BabataResult<Self> {
        let conn = Connection::open(path).map_err(|err| {
            BabataError::memory(format!(
                "Failed to open message db '{}': {}",
                path.display(),
                err
            ))
        })?;

        // Set jieba dictionary for this connection
        libsimple::set_jieba_dict(&conn, jieba_dir)
            .map_err(|e| BabataError::memory(format!("Failed to set jieba dict: {}", e)))?;

        // Enable WAL mode for better concurrency and set synchronous to NORMAL
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;
            PRAGMA foreign_keys=ON;",
        )
        .map_err(|e| BabataError::memory(format!("Failed to set PRAGMA: {}", e)))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY,
                role TEXT NOT NULL,
                message_type TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|e| BabataError::memory(format!("Failed to create messages table: {}", e)))?;

        conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
                content='messages',
                content_rowid='id',
                content,
                tokenize='simple'
            )",
            [],
        )
        .map_err(|e| BabataError::memory(format!("Failed to create FTS5 table: {}", e)))?;

        // Dynamic vector table creation with configurable dimension
        let vec_table_sql = format!(
            "CREATE VIRTUAL TABLE IF NOT EXISTS vec_messages USING vec0(
                message_id INTEGER PRIMARY KEY,
                embedding FLOAT[{}]
            )",
            dimension
        );
        conn.execute(&vec_table_sql, [])
            .map_err(|e| BabataError::memory(format!("Failed to create vector table: {}", e)))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_messages_created
             ON messages(created_at)",
            [],
        )
        .map_err(|e| BabataError::memory(format!("Failed to create index: {}", e)))?;

        // Index for filtering by role and message_type
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_messages_role_type
             ON messages(role, message_type)",
            [],
        )
        .map_err(|e| BabataError::memory(format!("Failed to create index: {}", e)))?;

        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages
             BEGIN
                 INSERT INTO messages_fts(rowid, content)
                 VALUES (new.id, new.content);
             END",
            [],
        )
        .map_err(|e| BabataError::memory(format!("Failed to create trigger: {}", e)))?;

        Ok(Self { conn, dimension })
    }

    pub fn insert_message(
        &mut self,
        role: &str,
        message_type: &str,
        content: &str,
        embedding: &[f32],
    ) -> BabataResult<i64> {
        if embedding.len() != self.dimension {
            return Err(BabataError::memory(format!(
                "Embedding dimension mismatch: expected {}, got {}",
                self.dimension,
                embedding.len()
            )));
        }

        let tx = self
            .conn
            .transaction()
            .map_err(|e| BabataError::memory(format!("Failed to begin transaction: {}", e)))?;

        tx.execute(
            "INSERT INTO messages (role, message_type, content, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![role, message_type, content, chrono::Utc::now().timestamp(),],
        )
        .map_err(|e| BabataError::memory(format!("Failed to insert message: {}", e)))?;

        let message_id = tx.last_insert_rowid();

        let blob = serialize_vector(embedding);
        tx.execute(
            "INSERT INTO vec_messages (message_id, embedding) VALUES (?1, ?2)",
            params![message_id, blob],
        )
        .map_err(|e| BabataError::memory(format!("Failed to insert embedding: {}", e)))?;

        tx.commit()
            .map_err(|e| BabataError::memory(format!("Failed to commit transaction: {}", e)))?;

        Ok(message_id)
    }

    pub fn bm25_search(&self, query: &str, limit: usize) -> BabataResult<Vec<BM25Result>> {
        let sql = "SELECT
                fts.rowid as message_id,
                m.content,
                bm25(messages_fts) as score,
                snippet(messages_fts, 0, '<mark>', '</mark>', '...', 32) as snippet
             FROM messages_fts fts
             JOIN messages m ON fts.rowid = m.id
             WHERE messages_fts MATCH jieba_query(?1)
             ORDER BY score
             LIMIT ?2";

        let mut stmt = self
            .conn
            .prepare(sql)
            .map_err(|e| BabataError::memory(format!("Failed to prepare BM25 query: {}", e)))?;

        let mapper = |row: &rusqlite::Row| {
            Ok(BM25Result {
                message_id: row.get(0)?,
                content: row.get(1)?,
                score: row.get(2)?,
                snippet: row.get(3)?,
            })
        };

        let results = stmt
            .query_map(params![query, limit], mapper)
            .map_err(|e| BabataError::memory(format!("Failed to execute BM25 query: {}", e)))?;

        results
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| BabataError::memory(format!("Failed to collect BM25 results: {}", e)))
    }

    pub fn vector_search(
        &self,
        query_embedding: &[f32],
        k: usize,
    ) -> BabataResult<Vec<VectorResult>> {
        let query_blob = serialize_vector(query_embedding);

        let sql = "SELECT
                v.message_id,
                m.content,
                distance
             FROM vec_messages v
             JOIN messages m ON v.message_id = m.id
             WHERE v.embedding MATCH ?1
             AND k = ?2
             ORDER BY distance";

        let mut stmt = self
            .conn
            .prepare(sql)
            .map_err(|e| BabataError::memory(format!("Failed to prepare vector query: {}", e)))?;

        let mapper = |row: &rusqlite::Row| {
            Ok(VectorResult {
                message_id: row.get(0)?,
                content: row.get(1)?,
                distance: row.get(2)?,
            })
        };

        let results = stmt
            .query_map(params![query_blob, k], mapper)
            .map_err(|e| BabataError::memory(format!("Failed to execute vector query: {}", e)))?;

        results
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| BabataError::memory(format!("Failed to collect vector results: {}", e)))
    }
}

#[derive(Debug, Clone)]
pub struct BM25Result {
    pub message_id: i64,
    pub content: String,
    pub score: f64,
    pub snippet: String,
}

#[derive(Debug, Clone)]
pub struct VectorResult {
    pub message_id: i64,
    pub content: String,
    pub distance: f32,
}

fn serialize_vector(vec: &[f32]) -> &[u8] {
    bytemuck::cast_slice(vec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
    use std::sync::atomic::{AtomicU64, Ordering};

    fn temp_db_path() -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!(
            "babata_test_{}_{}_{}.db",
            std::process::id(),
            id,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn test_chinese_fts_and_vector_search() -> BabataResult<()> {
        let mut model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::BGEM3)
                .with_cache_dir(std::env::temp_dir().join("fastembed_cache")),
        )
        .expect("Failed to create embedding model");

        let dimension = 1024;
        let mut store = MemoryStore::new(temp_db_path(), dimension)?;

        let texts = [
            "中华人民共和国国歌",
            "周杰伦是一位著名歌手",
            "今天天气很好",
            "人工智能技术",
            "机器学习算法",
            "深度学习神经网络",
        ];
        let embeddings = model.embed(texts, None).unwrap();

        for (text, embedding) in texts.iter().zip(embeddings.iter()) {
            store.insert_message("user", "text", text, embedding)?;
        }

        // Test Chinese FTS
        let results = store.bm25_search("中华国歌", 10)?;
        assert!(!results.is_empty(), "Should find results for '中华国歌'");
        assert_eq!(results[0].content, "中华人民共和国国歌");

        let results = store.bm25_search("周杰伦", 10)?;
        assert!(!results.is_empty(), "Should find results for '周杰伦'");
        assert!(results[0].content.contains("周杰伦"));

        // Test vector search
        let query_embedding = model.embed(vec!["人工智能"], None).unwrap();
        let results = store.vector_search(&query_embedding[0], 3)?;

        assert!(!results.is_empty(), "Should find vector search results");
        assert_eq!(results[0].content, "人工智能技术");

        Ok(())
    }

    #[test]
    fn test_embedding_dimension_mismatch() {
        let mut store = MemoryStore::new(temp_db_path(), 4).unwrap();

        let wrong_embedding = [0.1, 0.2, 0.3];
        let result = store.insert_message("user", "text", "test", &wrong_embedding);

        assert!(result.is_err());
    }
}
