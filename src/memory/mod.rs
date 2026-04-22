mod store;

pub use store::MessageRecord;
pub use store::MessageStore;

use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::message::Content;
use crate::{BabataResult, error::BabataError, message::Message};

#[derive(Debug)]
pub struct Memory {
    store: MessageStore,
    agent_home: PathBuf,
}

impl Memory {
    const CONTEXT_LIMIT: usize = 50;
    const TOOL_RESULT_CHAR_LIMIT: usize = 1_000;

    pub fn new(agent_home: impl AsRef<Path>) -> BabataResult<Self> {
        let agent_home = agent_home.as_ref().to_path_buf();
        let store = MessageStore::new(&agent_home)?;
        Self::ensure_memory_md(&agent_home)?;
        Ok(Self { store, agent_home })
    }

    fn ensure_memory_md(agent_home: impl AsRef<Path>) -> BabataResult<()> {
        let memory_dir = agent_home.as_ref().join("memory");
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

*This file is automatically updated by agent when important information should be remembered.*"#;

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

    pub fn append_messages(&self, task_id: Uuid, messages: &[Message]) -> BabataResult<()> {
        self.store.append_messages(task_id, messages)
    }

    pub fn scan_task_message_records(
        &self,
        task_id: Uuid,
        offset: usize,
        limit: usize,
    ) -> BabataResult<Vec<MessageRecord>> {
        self.store.scan_task_message_records(task_id, offset, limit)
    }

    fn render_context(messages: &[Message]) -> String {
        let filtered: Vec<&Message> = messages
            .iter()
            .filter(|m| !matches!(m, Message::AssistantThinking { .. }))
            .collect();

        if filtered.is_empty() {
            return String::new();
        }

        let mut sections = Vec::with_capacity(filtered.len() + 1);
        sections.push("## Conversation History".to_string());
        for message in filtered {
            sections.push(Self::render_message(message));
        }
        sections.join("\n\n")
    }

    fn render_message(message: &Message) -> String {
        match message {
            Message::UserPrompt { content, .. } => {
                format!("[user]\n{}", Self::render_content(content))
            }
            Message::UserSteering { content, .. } => {
                format!("[steer]\n{}", Self::render_content(content))
            }
            Message::AssistantResponse { content, .. } => {
                ["[assistant]".to_string(), Self::render_content(content)].join("\n")
            }
            Message::AssistantToolCalls { calls, .. } => {
                let mut lines = Vec::new();
                lines.push("[assistant_tool_calls]".to_string());
                for call in calls {
                    lines.push(format!("Tool: {}", call.tool_name));
                    lines.push(format!("Args: {}", call.args));
                }
                lines.join("\n")
            }
            Message::AssistantThinking { .. } => {
                // AssistantThinking is filtered out of context, but handle it here for completeness.
                String::new()
            }
            Message::ToolResult { call, result, .. } => {
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

    pub async fn build_context(&self, _prompts: &[Content]) -> BabataResult<String> {
        let mut context_parts = Vec::new();

        // Load long-term memory from MEMORY.md
        let memory_md_path = self.agent_home.join("memory").join("MEMORY.md");
        if memory_md_path.exists() {
            let long_term_memory = std::fs::read_to_string(&memory_md_path).map_err(|err| {
                BabataError::memory(format!(
                    "Failed to read MEMORY.md '{}': {}",
                    memory_md_path.display(),
                    err
                ))
            })?;
            if !long_term_memory.is_empty() {
                context_parts.push(format!(
                    "# Long-term Memory (from {})
\n{}",
                    memory_md_path.display(),
                    long_term_memory
                ));
            }
        }

        // Load short-term conversation history
        let messages = self.store.scan_recent_messages(Self::CONTEXT_LIMIT)?;
        let conversation_history = Self::render_context(&messages);
        if !conversation_history.is_empty() {
            context_parts.push(conversation_history);
        }

        Ok(context_parts.join("\n\n"))
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::message::{Content, Message};

    use super::*;

    #[test]
    fn message_json_has_type_tag() {
        let message = Message::UserPrompt {
            content: vec![Content::Text {
                text: "hello".to_string(),
            }],
            created_at: Utc::now(),
        };

        let payload = serde_json::to_value(&message).expect("serialize message into json");
        assert_eq!(payload["type"], "user_prompt");
    }

    #[tokio::test]
    async fn build_context_renders_messages_as_text() {
        let agent_home = std::env::temp_dir()
            .join("babata-tests")
            .join(format!("agent-home-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&agent_home).expect("create agent home directory");

        let store =
            MessageStore::open(agent_home.join("message.db")).expect("open sqlite message store");
        let memory = Memory { store, agent_home };

        let task_id = Uuid::new_v4();
        let now = Utc::now();
        memory
            .append_messages(
                task_id,
                &[
                    Message::UserPrompt {
                        content: vec![Content::Text {
                            text: "hello".to_string(),
                        }],
                        created_at: now,
                    },
                    Message::AssistantResponse {
                        content: vec![Content::Text {
                            text: "world".to_string(),
                        }],
                        created_at: now,
                    },
                ],
            )
            .expect("insert messages into sqlite");

        let context = memory
            .build_context(&[])
            .await
            .expect("build context from sqlite");

        assert!(context.contains("## Conversation History"));
        assert!(context.contains("[user]\nhello"));
        assert!(context.contains("[assistant]\nworld"));
    }
}
