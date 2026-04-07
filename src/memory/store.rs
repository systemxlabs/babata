use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};

use crate::{BabataResult, error::BabataError, message::Message};

#[derive(Debug)]
pub struct MessageStore {
    db_path: PathBuf,
}

impl MessageStore {
    pub fn new(agent_home: impl AsRef<Path>) -> BabataResult<Self> {
        let memory_dir = agent_home.as_ref().join("memory");
        std::fs::create_dir_all(&memory_dir).map_err(|err| {
            BabataError::memory(format!(
                "Failed to create memory directory '{}': {}",
                memory_dir.display(),
                err
            ))
        })?;
        let db_path = memory_dir.join("message.db");
        Self::open(db_path)
    }

    pub fn open(db_path: impl AsRef<Path>) -> BabataResult<Self> {
        let db_path = db_path.as_ref().to_path_buf();
        let Some(parent) = db_path.parent() else {
            return Err(BabataError::memory(format!(
                "Invalid sqlite path '{}'",
                db_path.display()
            )));
        };

        std::fs::create_dir_all(parent).map_err(|err| {
            BabataError::memory(format!(
                "Failed to create message db directory '{}': {}",
                parent.display(),
                err
            ))
        })?;

        let conn = Connection::open(&db_path).map_err(|err| {
            BabataError::memory(format!(
                "Failed to open message db '{}': {}",
                db_path.display(),
                err
            ))
        })?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                role TEXT NOT NULL,
                message TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            [],
        )
        .map_err(|err| {
            BabataError::memory(format!("Failed to initialize messages table: {}", err))
        })?;

        Ok(Self { db_path })
    }

    fn connect(&self) -> BabataResult<Connection> {
        Connection::open(&self.db_path).map_err(|err| {
            BabataError::memory(format!(
                "Failed to open message db '{}': {}",
                self.db_path.display(),
                err
            ))
        })
    }

    pub fn append_messages(&self, messages: &[Message]) -> BabataResult<()> {
        if messages.is_empty() {
            return Ok(());
        }

        let conn = self.connect()?;
        let mut stmt = conn
            .prepare("INSERT INTO messages (role, message) VALUES (?1, ?2)")
            .map_err(|err| {
                BabataError::memory(format!(
                    "Failed to prepare message insert statement: {}",
                    err
                ))
            })?;

        for message in messages {
            let role = message.role().to_string();
            let payload = serde_json::to_string(message).map_err(|err| {
                BabataError::memory(format!(
                    "Failed to serialize message payload into JSON: {}",
                    err
                ))
            })?;
            stmt.execute(params![role, payload]).map_err(|err| {
                BabataError::memory(format!("Failed to insert message row: {}", err))
            })?;
        }

        Ok(())
    }

    /// Scan recent messages ordered by time (oldest first among the recent ones).
    /// Returns empty vector if limit is 0.
    pub fn scan_recent_messages(&self, limit: usize) -> BabataResult<Vec<Message>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let query = "SELECT role, message FROM (
            SELECT role, message, created_at, rowid
            FROM messages
            ORDER BY datetime(created_at) DESC, rowid DESC
            LIMIT ?1
        )
        ORDER BY datetime(created_at), rowid";

        let limit_param = limit.min(i64::MAX as usize) as i64;

        let conn = self.connect()?;
        let mut stmt = conn.prepare(query).map_err(|err| {
            BabataError::memory(format!("Failed to prepare message scan statement: {}", err))
        })?;

        let mut rows = stmt.query(params![limit_param]).map_err(|err| {
            BabataError::memory(format!("Failed to query messages from sqlite: {}", err))
        })?;

        let mut messages = Vec::new();
        while let Some(row) = rows.next().map_err(|err| {
            BabataError::memory(format!("Failed to scan sqlite message row: {}", err))
        })? {
            let role: String = row.get(0).map_err(|err| {
                BabataError::memory(format!("Failed to read message role from row: {}", err))
            })?;
            let payload: String = row.get(1).map_err(|err| {
                BabataError::memory(format!("Failed to read message payload from row: {}", err))
            })?;

            let parsed: Message = serde_json::from_str(&payload).map_err(|err| {
                BabataError::memory(format!(
                    "Failed to parse message payload JSON '{}': {}",
                    payload, err
                ))
            })?;
            let expected = parsed.role().to_string();
            if role != expected {
                return Err(BabataError::memory(format!(
                    "Corrupted message row: role '{}' does not match message payload role '{}'",
                    role, expected
                )));
            }

            messages.push(parsed);
        }

        Ok(messages)
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::message::{Content, MediaType, Message, ToolCall};

    use super::*;

    #[test]
    fn insert_and_scan_messages_roundtrip() {
        let db_path = std::env::temp_dir()
            .join("babata-tests")
            .join(format!("message-store-{}.db", Uuid::new_v4()));

        let store = MessageStore::open(&db_path).expect("open sqlite message store");

        let messages = vec![
            Message::UserPrompt {
                content: vec![Content::Text {
                    text: "hello".to_string(),
                }],
            },
            Message::AssistantToolCalls {
                calls: vec![ToolCall {
                    call_id: "call-1".to_string(),
                    tool_name: "read_file".to_string(),
                    args: r#"{"path": "README.md"}"#.to_string(),
                }],
                reasoning_content: None,
            },
            Message::ToolResult {
                call: ToolCall {
                    call_id: "call-1".to_string(),
                    tool_name: "read_file".to_string(),
                    args: r#"{ "path": "README.md" }"#.to_string(),
                },
                result: "file content".to_string(),
            },
            Message::AssistantResponse {
                content: vec![
                    Content::Text {
                        text: "done".to_string(),
                    },
                    Content::ImageData {
                        data: "iVBORw0KGgoAAAANSUhEUgAAAAUA".to_string(),
                        media_type: MediaType::ImagePng,
                    },
                ],
                reasoning_content: None,
            },
        ];

        store
            .append_messages(&messages)
            .expect("insert messages into sqlite");
        let scanned = store
            .scan_recent_messages(messages.len())
            .expect("scan messages from sqlite");

        assert_eq!(messages, scanned);

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn scan_messages_with_limit_returns_latest_messages_in_order() {
        let db_path = std::env::temp_dir()
            .join("babata-tests")
            .join(format!("message-store-{}.db", Uuid::new_v4()));

        let store = MessageStore::open(&db_path).expect("open sqlite message store");
        let messages = vec![
            Message::UserPrompt {
                content: vec![Content::Text {
                    text: "m1".to_string(),
                }],
            },
            Message::UserPrompt {
                content: vec![Content::Text {
                    text: "m2".to_string(),
                }],
            },
            Message::UserPrompt {
                content: vec![Content::Text {
                    text: "m3".to_string(),
                }],
            },
        ];

        store
            .append_messages(&messages)
            .expect("insert messages into sqlite");

        let scanned = store
            .scan_recent_messages(2)
            .expect("scan limited messages from sqlite");
        assert_eq!(scanned.len(), 2);
        assert_eq!(scanned[0], messages[1]);
        assert_eq!(scanned[1], messages[2]);

        let scanned_empty = store
            .scan_recent_messages(0)
            .expect("scan zero messages from sqlite");
        assert!(scanned_empty.is_empty());

        let _ = std::fs::remove_file(db_path);
    }
}
