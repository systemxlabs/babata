use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};

use crate::message::Content;
use crate::utils::babata_dir;
use crate::{BabataResult, error::BabataError, memory::Memory, message::Message};

#[derive(Debug)]
pub struct SimpleMemory {
    db_path: PathBuf,
}

impl SimpleMemory {
    const CONTEXT_LIMIT: usize = 50;
    const TOOL_RESULT_CHAR_LIMIT: usize = 1_000;

    pub fn new() -> BabataResult<Self> {
        let db_path = Self::default_db_path()?;
        let memory = Self::open(db_path)?;
        Self::ensure_memory_md()?;
        Ok(memory)
    }

    fn ensure_memory_md() -> BabataResult<()> {
        let memory_dir = babata_dir()?.join("memory");
        std::fs::create_dir_all(&memory_dir).map_err(|err| {
            BabataError::memory(format!(
                "Failed to create memory directory '{}': {}",
                memory_dir.display(),
                err
            ))
        })?;

        let memory_md_path = memory_dir.join("MEMORY.md");
        if !memory_md_path.exists() {
            let initial_content = r#"# Long-term Memory

This file stores important information that should persist across sessions.

## User Information

(Important facts about the user)

## Preferences

(User preferences learned over time)

## Project Context

(Information about ongoing projects)

## Important Notes

(Things to remember)

---

*This file is automatically updated by babata when important information should be remembered.*"#;

            std::fs::write(&memory_md_path, initial_content).map_err(|err| {
                BabataError::memory(format!(
                    "Failed to create MEMORY.md '{}': {}",
                    memory_md_path.display(),
                    err
                ))
            })?;
        }

        Ok(())
    }

    fn open(db_path: impl AsRef<Path>) -> BabataResult<Self> {
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

    fn default_db_path() -> BabataResult<PathBuf> {
        let dir = babata_dir()?;
        Ok(dir.join("memory").join("message.db"))
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

    fn insert_messages_inner(&self, messages: &[Message]) -> BabataResult<()> {
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

    fn scan_messages(&self, limit: Option<usize>) -> BabataResult<Vec<Message>> {
        if limit == Some(0) {
            return Ok(Vec::new());
        }

        let (query, limit_param) = match limit {
            Some(limit) => (
                "SELECT role, message FROM (
                    SELECT role, message, created_at, rowid
                    FROM messages
                    ORDER BY datetime(created_at) DESC, rowid DESC
                    LIMIT ?1
                )
                ORDER BY datetime(created_at), rowid",
                Some(limit.min(i64::MAX as usize) as i64),
            ),
            None => (
                "SELECT role, message FROM messages ORDER BY datetime(created_at), rowid",
                None,
            ),
        };

        let conn = self.connect()?;
        let mut stmt = conn.prepare(query).map_err(|err| {
            BabataError::memory(format!("Failed to prepare message scan statement: {}", err))
        })?;

        let mut rows = match limit_param {
            Some(limit) => stmt.query(params![limit]).map_err(|err| {
                BabataError::memory(format!("Failed to query messages from sqlite: {}", err))
            })?,
            None => stmt.query([]).map_err(|err| {
                BabataError::memory(format!("Failed to query messages from sqlite: {}", err))
            })?,
        };

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

    fn render_context(messages: &[Message]) -> String {
        if messages.is_empty() {
            return String::new();
        }

        let mut sections = Vec::with_capacity(messages.len() + 1);
        sections.push("## Conversation History".to_string());
        for message in messages {
            sections.push(Self::render_message(message));
        }
        sections.join("\n\n")
    }

    fn render_message(message: &Message) -> String {
        match message {
            Message::UserPrompt { content } => {
                format!("[user]\n{}", Self::render_content(content))
            }
            Message::AssistantResponse {
                content,
                reasoning_content,
            } => {
                let mut lines = Vec::new();
                lines.push("[assistant]".to_string());
                if let Some(reasoning_content) = reasoning_content
                    && !reasoning_content.trim().is_empty()
                {
                    lines.push(format!("Reasoning:\n{}", reasoning_content.trim()));
                }
                lines.push(Self::render_content(content));
                lines.join("\n")
            }
            Message::AssistantToolCalls {
                calls,
                reasoning_content,
            } => {
                let mut lines = Vec::new();
                lines.push("[assistant_tool_calls]".to_string());
                if let Some(reasoning_content) = reasoning_content
                    && !reasoning_content.trim().is_empty()
                {
                    lines.push(format!("Reasoning:\n{}", reasoning_content.trim()));
                }
                for call in calls {
                    lines.push(format!("Tool: {}", call.tool_name));
                    lines.push(format!("Args: {}", call.args));
                }
                lines.join("\n")
            }
            Message::ToolResult { call, result } => {
                format!(
                    "[tool_result:{}]\n{}",
                    call.tool_name,
                    Self::truncate_text(result, Self::TOOL_RESULT_CHAR_LIMIT)
                )
            }
        }
    }

    fn render_content(content: &[Content]) -> String {
        if content.is_empty() {
            return "[empty]".to_string();
        }

        content
            .iter()
            .map(|part| match part {
                Content::Text { text } => text.clone(),
                Content::ImageUrl { url } => format!("[image_url] {url}"),
                Content::ImageData { media_type, .. } => {
                    format!("[image_data] {}", media_type.as_mime_str())
                }
                Content::AudioData { media_type, .. } => {
                    format!("[audio_data] {}", media_type.as_mime_str())
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn truncate_text(text: &str, limit: usize) -> String {
        if text.chars().count() <= limit {
            return text.to_string();
        }

        let truncated: String = text.chars().take(limit).collect();
        format!("{truncated}...")
    }
}

#[async_trait::async_trait]
impl Memory for SimpleMemory {
    fn name() -> &'static str {
        "simple"
    }

    async fn append_messages(&self, messages: Vec<Message>) -> BabataResult<()> {
        self.insert_messages_inner(&messages)
    }

    async fn build_context(&self, _prompts: &[Content]) -> BabataResult<String> {
        let mut context_parts = Vec::new();

        // Load long-term memory from MEMORY.md
        let memory_md_path = babata_dir()?.join("memory").join("MEMORY.md");
        if memory_md_path.exists() {
            match std::fs::read_to_string(&memory_md_path) {
                Ok(long_term_memory) => {
                    if !long_term_memory.is_empty() {
                        context_parts.push(format!(
                            "# Long-term Memory (from {})\n\n{}",
                            memory_md_path.display(),
                            long_term_memory
                        ));
                    }
                }
                Err(err) => {
                    eprintln!("Warning: Failed to read MEMORY.md: {}", err);
                }
            }
        }

        // Load short-term conversation history
        let messages = self.scan_messages(Some(Self::CONTEXT_LIMIT))?;
        let conversation_history = Self::render_context(&messages);
        if !conversation_history.is_empty() {
            context_parts.push(conversation_history);
        }

        Ok(context_parts.join("\n\n"))
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

        let memory = SimpleMemory::open(&db_path).expect("open sqlite message store");

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

        memory
            .insert_messages_inner(&messages)
            .expect("insert messages into sqlite");
        let scanned = memory
            .scan_messages(None)
            .expect("scan messages from sqlite");

        assert_eq!(messages, scanned);

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn scan_messages_with_limit_returns_latest_messages_in_order() {
        let db_path = std::env::temp_dir()
            .join("babata-tests")
            .join(format!("message-store-{}.db", Uuid::new_v4()));

        let memory = SimpleMemory::open(&db_path).expect("open sqlite message store");
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

        memory
            .insert_messages_inner(&messages)
            .expect("insert messages into sqlite");

        let scanned = memory
            .scan_messages(Some(2))
            .expect("scan limited messages from sqlite");
        assert_eq!(scanned.len(), 2);
        assert_eq!(scanned[0], messages[1]);
        assert_eq!(scanned[1], messages[2]);

        let scanned_empty = memory
            .scan_messages(Some(0))
            .expect("scan zero messages from sqlite");
        assert!(scanned_empty.is_empty());

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

    #[tokio::test]
    async fn build_context_renders_messages_as_text() {
        let db_path = std::env::temp_dir()
            .join("babata-tests")
            .join(format!("message-store-{}.db", Uuid::new_v4()));

        let memory = SimpleMemory::open(&db_path).expect("open sqlite message store");
        memory
            .insert_messages_inner(&[
                Message::UserPrompt {
                    content: vec![Content::Text {
                        text: "hello".to_string(),
                    }],
                },
                Message::AssistantResponse {
                    content: vec![Content::Text {
                        text: "world".to_string(),
                    }],
                    reasoning_content: None,
                },
            ])
            .expect("insert messages into sqlite");

        let context = memory
            .build_context(&[])
            .await
            .expect("build context from sqlite");

        assert!(context.contains("## Conversation History"));
        assert!(context.contains("[user]\nhello"));
        assert!(context.contains("[assistant]\nworld"));

        let _ = std::fs::remove_file(db_path);
    }
}
