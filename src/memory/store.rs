use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::{
    BabataResult,
    error::BabataError,
    message::{Content, Message, ToolCall},
};

/// Database record structure that maps 1:1 with the messages table schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageRecord {
    pub id: i64,
    pub message_type: MessageType,
    pub content: Option<String>,
    pub reasoning_content: Option<String>,
    pub tool_calls: Option<String>,
    pub call_id: Option<String>,
    pub tool_name: Option<String>,
    pub args: Option<String>,
    pub result: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    UserPrompt,
    UserSteering,
    AssistantResponse,
    AssistantToolCalls,
    ToolResult,
}

/// Database fields extracted from a Message for storage.
struct MessageFields {
    message_type: MessageType,
    content: Option<String>,
    reasoning_content: Option<String>,
    tool_calls: Option<String>,
    call_id: Option<String>,
    tool_name: Option<String>,
    args: Option<String>,
    result: Option<String>,
    created_at: String,
}

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
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                message_type TEXT NOT NULL,
                content TEXT,
                reasoning_content TEXT,
                tool_calls TEXT,
                call_id TEXT,
                tool_name TEXT,
                args TEXT,
                result TEXT,
                created_at TEXT NOT NULL
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

    /// Convert Message to database fields
    fn message_to_record_fields(message: &Message) -> BabataResult<MessageFields> {
        let created_at = message.created_at().to_rfc3339();

        let fields = match message {
            Message::UserPrompt { content: c, .. } => MessageFields {
                message_type: MessageType::UserPrompt,
                content: Some(serde_json::to_string(c).map_err(|e| {
                    BabataError::memory(format!("Failed to serialize content: {}", e))
                })?),
                reasoning_content: None,
                tool_calls: None,
                call_id: None,
                tool_name: None,
                args: None,
                result: None,
                created_at,
            },
            Message::UserSteering { content: c, .. } => MessageFields {
                message_type: MessageType::UserSteering,
                content: Some(serde_json::to_string(c).map_err(|e| {
                    BabataError::memory(format!("Failed to serialize content: {}", e))
                })?),
                reasoning_content: None,
                tool_calls: None,
                call_id: None,
                tool_name: None,
                args: None,
                result: None,
                created_at,
            },
            Message::AssistantResponse {
                content: c,
                reasoning_content: r,
                ..
            } => MessageFields {
                message_type: MessageType::AssistantResponse,
                content: Some(serde_json::to_string(c).map_err(|e| {
                    BabataError::memory(format!("Failed to serialize content: {}", e))
                })?),
                reasoning_content: r.clone(),
                tool_calls: None,
                call_id: None,
                tool_name: None,
                args: None,
                result: None,
                created_at,
            },
            Message::AssistantToolCalls {
                calls,
                reasoning_content: r,
                ..
            } => MessageFields {
                message_type: MessageType::AssistantToolCalls,
                content: None,
                reasoning_content: r.clone(),
                tool_calls: Some(serde_json::to_string(calls).map_err(|e| {
                    BabataError::memory(format!("Failed to serialize tool calls: {}", e))
                })?),
                call_id: None,
                tool_name: None,
                args: None,
                result: None,
                created_at,
            },
            Message::ToolResult {
                call, result: res, ..
            } => MessageFields {
                message_type: MessageType::ToolResult,
                content: None,
                reasoning_content: None,
                tool_calls: None,
                call_id: Some(call.call_id.clone()),
                tool_name: Some(call.tool_name.clone()),
                args: Some(call.args.clone()),
                result: Some(res.clone()),
                created_at,
            },
        };

        Ok(fields)
    }

    /// Convert database record to Message
    fn record_to_message(record: &MessageRecord) -> BabataResult<Message> {
        let created_at = record.created_at;

        let message = match record.message_type {
            MessageType::UserPrompt => {
                let content: Vec<Content> = record
                    .content
                    .as_ref()
                    .map(|c| {
                        serde_json::from_str(c).map_err(|e| {
                            BabataError::memory(format!("Failed to deserialize content: {}", e))
                        })
                    })
                    .transpose()?
                    .unwrap_or_default();
                Message::UserPrompt {
                    content,
                    created_at,
                }
            }
            MessageType::UserSteering => {
                let content: Vec<Content> = record
                    .content
                    .as_ref()
                    .map(|c| {
                        serde_json::from_str(c).map_err(|e| {
                            BabataError::memory(format!("Failed to deserialize content: {}", e))
                        })
                    })
                    .transpose()?
                    .unwrap_or_default();
                Message::UserSteering {
                    content,
                    created_at,
                }
            }
            MessageType::AssistantResponse => {
                let content: Vec<Content> = record
                    .content
                    .as_ref()
                    .map(|c| {
                        serde_json::from_str(c).map_err(|e| {
                            BabataError::memory(format!("Failed to deserialize content: {}", e))
                        })
                    })
                    .transpose()?
                    .unwrap_or_default();
                Message::AssistantResponse {
                    content,
                    reasoning_content: record.reasoning_content.clone(),
                    created_at,
                }
            }
            MessageType::AssistantToolCalls => {
                let calls: Vec<ToolCall> = record
                    .tool_calls
                    .as_ref()
                    .map(|c| {
                        serde_json::from_str(c).map_err(|e| {
                            BabataError::memory(format!("Failed to deserialize tool calls: {}", e))
                        })
                    })
                    .transpose()?
                    .unwrap_or_default();
                Message::AssistantToolCalls {
                    calls,
                    reasoning_content: record.reasoning_content.clone(),
                    created_at,
                }
            }
            MessageType::ToolResult => {
                let call = ToolCall {
                    call_id: record.call_id.clone().unwrap_or_default(),
                    tool_name: record.tool_name.clone().unwrap_or_default(),
                    args: record.args.clone().unwrap_or_default(),
                };
                Message::ToolResult {
                    call,
                    result: record.result.clone().unwrap_or_default(),
                    created_at,
                }
            }
        };

        Ok(message)
    }

    pub fn append_messages(&self, messages: &[Message]) -> BabataResult<()> {
        if messages.is_empty() {
            return Ok(());
        }

        let conn = self.connect()?;
        let mut stmt = conn
            .prepare("INSERT INTO messages (message_type, content, reasoning_content, tool_calls, call_id, tool_name, args, result, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)")
            .map_err(|err| {
                BabataError::memory(format!(
                    "Failed to prepare message insert statement: {}",
                    err
                ))
            })?;

        for message in messages {
            let fields = Self::message_to_record_fields(message)?;
            let message_type_str = serde_json::to_string(&fields.message_type).map_err(|e| {
                BabataError::memory(format!("Failed to serialize message type: {}", e))
            })?;
            let message_type_str = message_type_str.trim_matches('"').to_string();

            stmt.execute(params![
                message_type_str,
                fields.content,
                fields.reasoning_content,
                fields.tool_calls,
                fields.call_id,
                fields.tool_name,
                fields.args,
                fields.result,
                fields.created_at
            ])
            .map_err(|err| BabataError::memory(format!("Failed to insert message row: {}", err)))?;
        }

        Ok(())
    }

    /// Scan recent messages ordered by time (oldest first among the recent ones).
    /// Returns empty vector if limit is 0.
    pub fn scan_recent_messages(&self, limit: usize) -> BabataResult<Vec<Message>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let query = "SELECT id, message_type, content, reasoning_content, tool_calls, call_id, tool_name, args, result, created_at FROM (
            SELECT id, message_type, content, reasoning_content, tool_calls, call_id, tool_name, args, result, created_at, rowid
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
            let id: i64 = row.get(0).map_err(|err| {
                BabataError::memory(format!("Failed to read id from row: {}", err))
            })?;
            let message_type_str: String = row.get(1).map_err(|err| {
                BabataError::memory(format!("Failed to read message_type from row: {}", err))
            })?;
            let content: Option<String> = row.get(2).map_err(|err| {
                BabataError::memory(format!("Failed to read content from row: {}", err))
            })?;
            let reasoning_content: Option<String> = row.get(3).map_err(|err| {
                BabataError::memory(format!(
                    "Failed to read reasoning_content from row: {}",
                    err
                ))
            })?;
            let tool_calls: Option<String> = row.get(4).map_err(|err| {
                BabataError::memory(format!("Failed to read tool_calls from row: {}", err))
            })?;
            let call_id: Option<String> = row.get(5).map_err(|err| {
                BabataError::memory(format!("Failed to read call_id from row: {}", err))
            })?;
            let tool_name: Option<String> = row.get(6).map_err(|err| {
                BabataError::memory(format!("Failed to read tool_name from row: {}", err))
            })?;
            let args: Option<String> = row.get(7).map_err(|err| {
                BabataError::memory(format!("Failed to read args from row: {}", err))
            })?;
            let result: Option<String> = row.get(8).map_err(|err| {
                BabataError::memory(format!("Failed to read result from row: {}", err))
            })?;
            let created_at_str: String = row.get(9).map_err(|err| {
                BabataError::memory(format!("Failed to read created_at from row: {}", err))
            })?;
            let created_at = created_at_str.parse::<DateTime<Utc>>().map_err(|err| {
                BabataError::memory(format!(
                    "Failed to parse created_at '{}': {}",
                    created_at_str, err
                ))
            })?;

            // Parse message_type from string
            let message_type: MessageType = match message_type_str.as_str() {
                "user_prompt" => MessageType::UserPrompt,
                "user_steering" => MessageType::UserSteering,
                "assistant_response" => MessageType::AssistantResponse,
                "assistant_tool_calls" => MessageType::AssistantToolCalls,
                "tool_result" => MessageType::ToolResult,
                _ => {
                    return Err(BabataError::memory(format!(
                        "Unknown message type: {}",
                        message_type_str
                    )));
                }
            };

            let record = MessageRecord {
                id,
                message_type,
                content,
                reasoning_content,
                tool_calls,
                call_id,
                tool_name,
                args,
                result,
                created_at,
            };

            let message = Self::record_to_message(&record)?;
            messages.push(message);
        }

        Ok(messages)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::message::{Content, MediaType, Message, ToolCall};

    use super::*;

    #[test]
    fn insert_and_scan_messages_roundtrip() {
        let db_path = std::env::temp_dir()
            .join("babata-tests")
            .join(format!("message-store-{}.db", Uuid::new_v4()));

        let store = MessageStore::open(&db_path).expect("open sqlite message store");

        let now = Utc::now();
        let messages = vec![
            Message::UserPrompt {
                content: vec![Content::Text {
                    text: "hello".to_string(),
                }],
                created_at: now,
            },
            Message::AssistantToolCalls {
                calls: vec![ToolCall {
                    call_id: "call-1".to_string(),
                    tool_name: "read_file".to_string(),
                    args: r#"{"path": "README.md"}"#.to_string(),
                }],
                reasoning_content: None,
                created_at: now,
            },
            Message::ToolResult {
                call: ToolCall {
                    call_id: "call-1".to_string(),
                    tool_name: "read_file".to_string(),
                    args: r#"{ "path": "README.md" }"#.to_string(),
                },
                result: "file content".to_string(),
                created_at: now,
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
                created_at: now,
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
        let now = Utc::now();
        let messages = vec![
            Message::UserPrompt {
                content: vec![Content::Text {
                    text: "m1".to_string(),
                }],
                created_at: now,
            },
            Message::UserPrompt {
                content: vec![Content::Text {
                    text: "m2".to_string(),
                }],
                created_at: now + chrono::Duration::seconds(1),
            },
            Message::UserPrompt {
                content: vec![Content::Text {
                    text: "m3".to_string(),
                }],
                created_at: now + chrono::Duration::seconds(2),
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
