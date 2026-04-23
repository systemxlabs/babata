use std::{
    fmt,
    path::{Path, PathBuf},
    str::FromStr,
};

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    BabataResult,
    error::BabataError,
    message::{Content, Message, ToolCall},
};

/// Database record structure that maps 1:1 with the messages table schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageRecord {
    pub task_id: Uuid,
    pub message_type: MessageType,
    #[serde(with = "option_json_string")]
    pub content: Option<Vec<Content>>,
    pub signature: Option<String>,
    #[serde(with = "option_json_string")]
    pub tool_calls: Option<Vec<ToolCall>>,
    pub result: Option<String>,
    pub created_at: DateTime<Utc>,
}

mod option_json_string {
    use serde::de::{DeserializeOwned, Visitor};
    use serde::{Deserializer, Serialize, Serializer};

    pub fn serialize<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
        S: Serializer,
    {
        match value {
            Some(v) => {
                let json = serde_json::to_string(v).map_err(serde::ser::Error::custom)?;
                serializer.serialize_some(&json)
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
    where
        T: DeserializeOwned,
        D: Deserializer<'de>,
    {
        struct JsonStringVisitor<T>(std::marker::PhantomData<T>);

        impl<'de, T> Visitor<'de> for JsonStringVisitor<T>
        where
            T: DeserializeOwned,
        {
            type Value = Option<T>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a JSON string or null")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let parsed = serde_json::from_str(value).map_err(serde::de::Error::custom)?;
                Ok(Some(parsed))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(None)
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserializer.deserialize_str(self)
            }
        }

        deserializer.deserialize_option(JsonStringVisitor(std::marker::PhantomData))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    UserPrompt,
    UserSteering,
    AssistantResponse,
    AssistantToolCalls,
    AssistantThinking,
    ToolResult,
}

impl fmt::Display for MessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            MessageType::UserPrompt => "user_prompt",
            MessageType::UserSteering => "user_steering",
            MessageType::AssistantResponse => "assistant_response",
            MessageType::AssistantToolCalls => "assistant_tool_calls",
            MessageType::AssistantThinking => "assistant_thinking",
            MessageType::ToolResult => "tool_result",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for MessageType {
    type Err = BabataError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user_prompt" => Ok(MessageType::UserPrompt),
            "user_steering" => Ok(MessageType::UserSteering),
            "assistant_response" => Ok(MessageType::AssistantResponse),
            "assistant_tool_calls" => Ok(MessageType::AssistantToolCalls),
            "assistant_thinking" => Ok(MessageType::AssistantThinking),
            "tool_result" => Ok(MessageType::ToolResult),
            _ => Err(BabataError::memory(format!("Unknown message type: {}", s))),
        }
    }
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
                task_id TEXT NOT NULL,
                message_type TEXT NOT NULL,
                content TEXT,
                signature TEXT,
                tool_calls TEXT,
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

    /// Convert Message to MessageRecord
    fn message_to_record(message: &Message, task_id: Uuid) -> BabataResult<MessageRecord> {
        let created_at = *message.created_at();

        let record = match message {
            Message::UserPrompt { content: c, .. } => MessageRecord {
                task_id,
                message_type: MessageType::UserPrompt,
                content: Some(c.clone()),
                signature: None,
                tool_calls: None,
                result: None,
                created_at,
            },
            Message::UserSteering { content: c, .. } => MessageRecord {
                task_id,
                message_type: MessageType::UserSteering,
                content: Some(c.clone()),
                signature: None,
                tool_calls: None,
                result: None,
                created_at,
            },
            Message::AssistantResponse { content: c, .. } => MessageRecord {
                task_id,
                message_type: MessageType::AssistantResponse,
                content: Some(c.clone()),
                signature: None,
                tool_calls: None,
                result: None,
                created_at,
            },
            Message::AssistantToolCalls { calls, .. } => MessageRecord {
                task_id,
                message_type: MessageType::AssistantToolCalls,
                content: None,
                signature: None,
                tool_calls: Some(calls.clone()),
                result: None,
                created_at,
            },
            Message::AssistantThinking {
                content, signature, ..
            } => MessageRecord {
                task_id,
                message_type: MessageType::AssistantThinking,
                content: Some(vec![Content::Text {
                    text: content.clone(),
                }]),
                signature: signature.clone(),
                tool_calls: None,
                result: None,
                created_at,
            },
            Message::ToolResult {
                call, result: res, ..
            } => MessageRecord {
                task_id,
                message_type: MessageType::ToolResult,
                content: None,
                signature: None,
                tool_calls: Some(vec![call.clone()]),
                result: Some(res.clone()),
                created_at,
            },
        };

        Ok(record)
    }

    /// Convert database record to Message
    fn record_to_message(record: &MessageRecord) -> BabataResult<Message> {
        let created_at = record.created_at;

        let message = match record.message_type {
            MessageType::UserPrompt => {
                let content = record.content.clone().unwrap_or_default();
                Message::UserPrompt {
                    content,
                    created_at,
                }
            }
            MessageType::UserSteering => {
                let content = record.content.clone().unwrap_or_default();
                Message::UserSteering {
                    content,
                    created_at,
                }
            }
            MessageType::AssistantResponse => {
                let content = record.content.clone().unwrap_or_default();
                Message::AssistantResponse {
                    content,
                    created_at,
                }
            }
            MessageType::AssistantToolCalls => {
                let calls = record.tool_calls.clone().unwrap_or_default();
                Message::AssistantToolCalls { calls, created_at }
            }
            MessageType::AssistantThinking => {
                let text = record
                    .content
                    .as_ref()
                    .and_then(|c| c.first())
                    .map(|c| match c {
                        Content::Text { text } => text.clone(),
                        _ => String::new(),
                    })
                    .unwrap_or_default();
                Message::AssistantThinking {
                    content: text,
                    signature: record.signature.clone(),
                    created_at,
                }
            }
            MessageType::ToolResult => {
                let call = record
                    .tool_calls
                    .as_ref()
                    .and_then(|calls| calls.first())
                    .cloned()
                    .unwrap_or(ToolCall {
                        call_id: String::new(),
                        tool_name: String::new(),
                        args: String::new(),
                    });
                Message::ToolResult {
                    call,
                    result: record.result.clone().unwrap_or_default(),
                    created_at,
                }
            }
        };

        Ok(message)
    }

    pub fn append_messages(&self, task_id: Uuid, messages: &[Message]) -> BabataResult<()> {
        if messages.is_empty() {
            return Ok(());
        }

        let conn = self.connect()?;
        let mut stmt = conn
            .prepare("INSERT INTO messages (task_id, message_type, content, signature, tool_calls, result, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")
            .map_err(|err| {
                BabataError::memory(format!(
                    "Failed to prepare message insert statement: {}",
                    err
                ))
            })?;

        for message in messages {
            let record = Self::message_to_record(message, task_id)?;
            let message_type_str = record.message_type.to_string();

            let content_json = record
                .content
                .as_ref()
                .map(|c| {
                    serde_json::to_string(c).map_err(|e| {
                        BabataError::memory(format!("Failed to serialize content: {}", e))
                    })
                })
                .transpose()?;
            let tool_calls_json = record
                .tool_calls
                .as_ref()
                .map(|c| {
                    serde_json::to_string(c).map_err(|e| {
                        BabataError::memory(format!("Failed to serialize tool calls: {}", e))
                    })
                })
                .transpose()?;

            stmt.execute(params![
                record.task_id.to_string(),
                message_type_str,
                content_json,
                record.signature,
                tool_calls_json,
                record.result,
                record.created_at.to_rfc3339()
            ])
            .map_err(|err| BabataError::memory(format!("Failed to insert message row: {}", err)))?;
        }

        Ok(())
    }

    pub fn scan_task_message_records(
        &self,
        task_id: Uuid,
        offset: usize,
        limit: usize,
        message_type: Option<MessageType>,
    ) -> BabataResult<Vec<MessageRecord>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let query = if message_type.is_some() {
            "SELECT task_id, message_type, content, signature, tool_calls, result, created_at
            FROM messages
            WHERE task_id = ?1 AND message_type = ?2
            ORDER BY datetime(created_at), rowid
            LIMIT ?3 OFFSET ?4"
        } else {
            "SELECT task_id, message_type, content, signature, tool_calls, result, created_at
            FROM messages
            WHERE task_id = ?1
            ORDER BY datetime(created_at), rowid
            LIMIT ?2 OFFSET ?3"
        };

        let task_id_param = task_id.to_string();
        let limit_param = limit.min(i64::MAX as usize) as i64;
        let offset_param = offset.min(i64::MAX as usize) as i64;

        let conn = self.connect()?;
        let mut stmt = conn.prepare(query).map_err(|err| {
            BabataError::memory(format!(
                "Failed to prepare task message scan statement: {}",
                err
            ))
        })?;

        let mut rows = if let Some(mt) = message_type {
            stmt.query(params![
                task_id_param,
                mt.to_string(),
                limit_param,
                offset_param
            ])
        } else {
            stmt.query(params![task_id_param, limit_param, offset_param])
        }
        .map_err(|err| {
            BabataError::memory(format!(
                "Failed to query task messages from sqlite: {}",
                err
            ))
        })?;

        let mut records = Vec::new();
        while let Some(row) = rows.next().map_err(|err| {
            BabataError::memory(format!("Failed to scan sqlite task message row: {}", err))
        })? {
            let task_id_str: String = row.get(0).map_err(|err| {
                BabataError::memory(format!("Failed to read task_id from row: {}", err))
            })?;
            let task_id = Uuid::parse_str(&task_id_str).map_err(|err| {
                BabataError::memory(format!(
                    "Failed to parse task_id '{}': {}",
                    task_id_str, err
                ))
            })?;
            let message_type_str: String = row.get(1).map_err(|err| {
                BabataError::memory(format!("Failed to read message_type from row: {}", err))
            })?;
            let content_json: Option<String> = row.get(2).map_err(|err| {
                BabataError::memory(format!("Failed to read content from row: {}", err))
            })?;
            let signature: Option<String> = row.get(3).map_err(|err| {
                BabataError::memory(format!("Failed to read signature from row: {}", err))
            })?;
            let tool_calls_json: Option<String> = row.get(4).map_err(|err| {
                BabataError::memory(format!("Failed to read tool_calls from row: {}", err))
            })?;
            let result: Option<String> = row.get(5).map_err(|err| {
                BabataError::memory(format!("Failed to read result from row: {}", err))
            })?;
            let created_at_str: String = row.get(6).map_err(|err| {
                BabataError::memory(format!("Failed to read created_at from row: {}", err))
            })?;
            let created_at = created_at_str.parse::<DateTime<Utc>>().map_err(|err| {
                BabataError::memory(format!(
                    "Failed to parse created_at '{}': {}",
                    created_at_str, err
                ))
            })?;

            let message_type: MessageType = message_type_str.parse()?;
            let content: Option<Vec<Content>> = content_json
                .as_ref()
                .map(|c| {
                    serde_json::from_str(c).map_err(|e| {
                        BabataError::memory(format!("Failed to deserialize content: {}", e))
                    })
                })
                .transpose()?;
            let tool_calls: Option<Vec<ToolCall>> = tool_calls_json
                .as_ref()
                .map(|c| {
                    serde_json::from_str(c).map_err(|e| {
                        BabataError::memory(format!("Failed to deserialize tool_calls: {}", e))
                    })
                })
                .transpose()?;

            records.push(MessageRecord {
                task_id,
                message_type,
                content,
                signature,
                tool_calls,
                result,
                created_at,
            });
        }

        Ok(records)
    }

    /// Scan recent messages ordered by time (oldest first among the recent ones).
    /// Returns empty vector if limit is 0.
    pub fn scan_recent_messages(&self, limit: usize) -> BabataResult<Vec<Message>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let query =
            "SELECT task_id, message_type, content, signature, tool_calls, result, created_at FROM (
            SELECT task_id, message_type, content, signature, tool_calls, result, created_at, rowid
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
            let task_id_str: String = row.get(0).map_err(|err| {
                BabataError::memory(format!("Failed to read task_id from row: {}", err))
            })?;
            let task_id = Uuid::parse_str(&task_id_str).map_err(|err| {
                BabataError::memory(format!(
                    "Failed to parse task_id '{}': {}",
                    task_id_str, err
                ))
            })?;
            let message_type_str: String = row.get(1).map_err(|err| {
                BabataError::memory(format!("Failed to read message_type from row: {}", err))
            })?;
            let content_json: Option<String> = row.get(2).map_err(|err| {
                BabataError::memory(format!("Failed to read content from row: {}", err))
            })?;
            let signature: Option<String> = row.get(3).map_err(|err| {
                BabataError::memory(format!("Failed to read signature from row: {}", err))
            })?;
            let tool_calls_json: Option<String> = row.get(4).map_err(|err| {
                BabataError::memory(format!("Failed to read tool_calls from row: {}", err))
            })?;
            let result: Option<String> = row.get(5).map_err(|err| {
                BabataError::memory(format!("Failed to read result from row: {}", err))
            })?;
            let created_at_str: String = row.get(6).map_err(|err| {
                BabataError::memory(format!("Failed to read created_at from row: {}", err))
            })?;
            let created_at = created_at_str.parse::<DateTime<Utc>>().map_err(|err| {
                BabataError::memory(format!(
                    "Failed to parse created_at '{}': {}",
                    created_at_str, err
                ))
            })?;

            // Parse message_type from string
            let message_type: MessageType = message_type_str.parse()?;

            // Deserialize content and tool_calls from JSON strings
            let content: Option<Vec<Content>> = content_json
                .as_ref()
                .map(|c| {
                    serde_json::from_str(c).map_err(|e| {
                        BabataError::memory(format!("Failed to deserialize content: {}", e))
                    })
                })
                .transpose()?;
            let tool_calls: Option<Vec<ToolCall>> = tool_calls_json
                .as_ref()
                .map(|c| {
                    serde_json::from_str(c).map_err(|e| {
                        BabataError::memory(format!("Failed to deserialize tool_calls: {}", e))
                    })
                })
                .transpose()?;

            let record = MessageRecord {
                task_id,
                message_type,
                content,
                signature,
                tool_calls,
                result,
                created_at,
            };

            let message = Self::record_to_message(&record)?;
            messages.push(message);
        }

        Ok(messages)
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

        let task_id = Uuid::new_v4();
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
                created_at: now,
            },
            Message::AssistantThinking {
                content: "Let me think...".to_string(),
                signature: Some("sig-abc".to_string()),
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
                created_at: now,
            },
        ];

        store
            .append_messages(task_id, &messages)
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
        let task_id = Uuid::new_v4();
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
            .append_messages(task_id, &messages)
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

    #[test]
    fn scan_task_message_records_returns_one_task_in_ascending_order_with_pagination() {
        let db_path = std::env::temp_dir()
            .join("babata-tests")
            .join(format!("message-store-{}.db", Uuid::new_v4()));

        let store = MessageStore::open(&db_path).expect("open sqlite message store");
        let task_id = Uuid::new_v4();
        let other_task_id = Uuid::new_v4();
        let now = Utc::now();

        store
            .append_messages(
                task_id,
                &[
                    Message::UserPrompt {
                        content: vec![Content::Text {
                            text: "m1".to_string(),
                        }],
                        created_at: now,
                    },
                    Message::UserSteering {
                        content: vec![Content::Text {
                            text: "m2".to_string(),
                        }],
                        created_at: now + chrono::Duration::seconds(1),
                    },
                    Message::AssistantResponse {
                        content: vec![Content::Text {
                            text: "m3".to_string(),
                        }],
                        created_at: now + chrono::Duration::seconds(2),
                    },
                ],
            )
            .expect("insert task messages");

        store
            .append_messages(
                other_task_id,
                &[Message::UserPrompt {
                    content: vec![Content::Text {
                        text: "other".to_string(),
                    }],
                    created_at: now + chrono::Duration::seconds(3),
                }],
            )
            .expect("insert other task message");

        let records = store
            .scan_task_message_records(task_id, 1, 2, None)
            .expect("scan paginated task message records");

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].task_id, task_id);
        assert_eq!(records[0].message_type, MessageType::UserSteering);
        assert_eq!(records[1].task_id, task_id);
        assert_eq!(records[1].message_type, MessageType::AssistantResponse);

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn scan_task_message_records_with_message_type_filter() {
        let db_path = std::env::temp_dir()
            .join("babata-tests")
            .join(format!("message-store-filter-{}.db", Uuid::new_v4()));

        let store = MessageStore::open(&db_path).expect("open sqlite message store");
        let task_id = Uuid::new_v4();
        let now = Utc::now();

        store
            .append_messages(
                task_id,
                &[
                    Message::UserPrompt {
                        content: vec![Content::Text {
                            text: "user message 1".to_string(),
                        }],
                        created_at: now,
                    },
                    Message::AssistantResponse {
                        content: vec![Content::Text {
                            text: "assistant response 1".to_string(),
                        }],
                        created_at: now + chrono::Duration::seconds(1),
                    },
                    Message::UserPrompt {
                        content: vec![Content::Text {
                            text: "user message 2".to_string(),
                        }],
                        created_at: now + chrono::Duration::seconds(2),
                    },
                    Message::AssistantToolCalls {
                        calls: vec![ToolCall {
                            call_id: "call-1".to_string(),
                            tool_name: "read_file".to_string(),
                            args: r#"{"path": "README.md"}"#.to_string(),
                        }],
                        created_at: now + chrono::Duration::seconds(3),
                    },
                ],
            )
            .expect("insert messages");

        // Filter by user_prompt - should return 2 messages
        let user_records = store
            .scan_task_message_records(task_id, 0, 10, Some(MessageType::UserPrompt))
            .expect("scan with user_prompt filter");
        assert_eq!(user_records.len(), 2);
        assert!(
            user_records
                .iter()
                .all(|r| r.message_type == MessageType::UserPrompt)
        );

        // Filter by assistant_response - should return 1 message
        let assistant_records = store
            .scan_task_message_records(task_id, 0, 10, Some(MessageType::AssistantResponse))
            .expect("scan with assistant_response filter");
        assert_eq!(assistant_records.len(), 1);
        assert_eq!(
            assistant_records[0].message_type,
            MessageType::AssistantResponse
        );

        // Filter by assistant_tool_calls - should return 1 message
        let tool_call_records = store
            .scan_task_message_records(task_id, 0, 10, Some(MessageType::AssistantToolCalls))
            .expect("scan with assistant_tool_calls filter");
        assert_eq!(tool_call_records.len(), 1);
        assert_eq!(
            tool_call_records[0].message_type,
            MessageType::AssistantToolCalls
        );

        // No filter - should return all 4 messages
        let all_records = store
            .scan_task_message_records(task_id, 0, 10, None)
            .expect("scan without filter");
        assert_eq!(all_records.len(), 4);

        // Filter with pagination
        let paginated_user_records = store
            .scan_task_message_records(task_id, 1, 10, Some(MessageType::UserPrompt))
            .expect("scan with filter and offset");
        assert_eq!(paginated_user_records.len(), 1);
        assert_eq!(
            paginated_user_records[0].message_type,
            MessageType::UserPrompt
        );

        let _ = std::fs::remove_file(db_path);
    }
}
