use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};

use crate::utils::babata_dir;
use crate::{BabataResult, error::BabataError, message::Message};

pub struct MessageStore {
    conn: Connection,
}

impl MessageStore {
    pub fn new() -> BabataResult<Self> {
        let db_path = Self::default_db_path()?;
        Self::open(db_path)
    }

    fn default_db_path() -> BabataResult<PathBuf> {
        let dir = babata_dir()?;
        Ok(dir.join("message.db"))
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
                "Failed to create message db directory '{}': {}",
                parent.display(),
                err
            ))
        })?;

        let conn = Connection::open(path).map_err(|err| {
            BabataError::memory(format!(
                "Failed to open message db '{}': {}",
                path.display(),
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

        Ok(Self { conn })
    }

    pub fn insert_messages(&self, messages: &[Message]) -> BabataResult<()> {
        if messages.is_empty() {
            return Ok(());
        }

        let mut stmt = self
            .conn
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

    pub fn scan_messages(&self) -> BabataResult<Vec<Message>> {
        let mut stmt = self
            .conn
            .prepare("SELECT role, message FROM messages ORDER BY datetime(created_at), rowid")
            .map_err(|err| {
                BabataError::memory(format!("Failed to prepare message scan statement: {}", err))
            })?;

        let mut rows = stmt.query([]).map_err(|err| {
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

    use crate::message::{Content, Message, ToolCall};

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
                        media_type: "image/png".to_string(),
                    },
                ],
            },
        ];

        store
            .insert_messages(&messages)
            .expect("insert messages into sqlite");
        let scanned = store.scan_messages().expect("scan messages from sqlite");

        assert_eq!(messages, scanned);

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn message_json_has_type_tag() {
        let message = Message::UserPrompt {
            content: vec![Content::Text {
                text: "hello".to_string(),
            }],
        };

        let payload = serde_json::to_value(&message).expect("serialize message into json");
        assert_eq!(payload["type"], "user_prompt");
    }
}
