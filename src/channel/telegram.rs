use std::collections::HashSet;

use teloxide::{
    payloads::GetUpdatesSetters,
    prelude::{Request, Requester},
    types::{ChatId, ChatKind, Update, UpdateKind},
    Bot,
};
use tokio::sync::Mutex;

use crate::{
    BabataResult,
    config::{ChannelConfig, Config},
    error::BabataError,
    message::{Content, Message},
};

#[derive(Debug)]
pub struct TelegramChannel {
    bot: Bot,
    // Long-poll timeout used by blocking receive().
    polling_timeout_secs: u64,
    // Telegram update cursor to avoid reprocessing already consumed updates.
    last_update_id: Mutex<Option<i64>>,
    // Allowed DM user ids; messages from others are ignored.
    allowed_user_ids: HashSet<i64>,
}

impl TelegramChannel {
    pub fn new(bot_token: &str) -> Self {
        Self {
            bot: Bot::new(bot_token),
            polling_timeout_secs: 30,
            last_update_id: Mutex::new(None),
            allowed_user_ids: HashSet::new(),
        }
    }

    pub fn with_polling_timeout_secs(mut self, polling_timeout_secs: u64) -> Self {
        self.polling_timeout_secs = polling_timeout_secs;
        self
    }

    pub fn with_last_update_id(mut self, last_update_id: Option<i64>) -> Self {
        self.last_update_id = Mutex::new(last_update_id);
        self
    }

    pub fn with_allowed_user_ids(mut self, allowed_user_ids: Vec<i64>) -> Self {
        self.allowed_user_ids = allowed_user_ids.into_iter().collect();
        self
    }

    async fn fetch_updates(&self, timeout_secs: u64) -> BabataResult<Vec<IncomingPrivateMessage>> {
        let offset = *self.last_update_id.lock().await;
        let mut request = self.bot.get_updates().timeout(timeout_secs as u32);

        if let Some(offset) = offset {
            // Telegram offset is i32 in teloxide API; saturate safely from stored i64.
            let next = offset.saturating_add(1);
            let next = next.clamp(i32::MIN as i64, i32::MAX as i64) as i32;
            request = request.offset(next);
        }

        let updates = request.send().await.map_err(|err| {
            BabataError::internal(format!("Failed to call Telegram getUpdates: {err}"))
        })?;

        // Keep only DM messages and return the max update_id for cursor advancing.
        let (max_update_id, messages) = extract_private_messages(updates, &self.allowed_user_ids);

        if let Some(max_update_id) = max_update_id {
            self.update_last_update_id(max_update_id).await?;
        }

        Ok(messages)
    }

    async fn update_last_update_id(&self, last_update_id: i64) -> BabataResult<()> {
        {
            let mut current = self.last_update_id.lock().await;
            if current.is_some_and(|current_id| current_id >= last_update_id) {
                return Ok(());
            }
            *current = Some(last_update_id);
        }

        self.persist_last_update_id(last_update_id)
    }

    fn persist_last_update_id(&self, last_update_id: i64) -> BabataResult<()> {
        let mut config = Config::load()?;
        let mut updated = false;
        if let Some(channel) = config.channels.iter_mut().next() {
            let ChannelConfig::Telegram(telegram) = channel;
            telegram.last_update_id = Some(last_update_id);
            updated = true;
        }

        if updated {
            config.save()?;
        }

        Ok(())
    }

    async fn send_text(&self, chat_id: i64, text: &str) -> BabataResult<()> {
        self.bot
            .send_message(ChatId(chat_id), text.to_string())
            .send()
            .await
            .map_err(|err| {
                BabataError::internal(format!("Failed to call Telegram sendMessage: {err}"))
            })?;

        Ok(())
    }

    async fn incoming_to_messages(
        &self,
        incoming: Vec<IncomingPrivateMessage>,
    ) -> Option<Vec<Message>> {
        if incoming.is_empty() {
            return None;
        }

        let messages = incoming
            .into_iter()
            .filter(|m| self.allowed_user_ids.contains(&m.chat_id))
            .map(|message| Message::UserPrompt {
                content: vec![Content::Text { text: message.text }],
            })
            .collect::<Vec<_>>();

        if messages.is_empty() {
            return None;
        }

        Some(messages)
    }
}

#[async_trait::async_trait]
impl super::Channel for TelegramChannel {
    fn name() -> &'static str {
        "Telegram"
    }

    async fn send(&self, messages: &[Message]) -> BabataResult<()> {
        let outgoing = extract_outgoing_texts(messages);
        for text in outgoing {
            for chat_id in &self.allowed_user_ids {
                self.send_text(*chat_id, &text).await?;
            }
        }

        Ok(())
    }

    async fn receive(&self) -> BabataResult<Vec<Message>> {
        loop {
            // Blocking long-poll until Telegram returns new updates.
            let incoming = self.fetch_updates(self.polling_timeout_secs).await?;
            if let Some(messages) = self.incoming_to_messages(incoming).await {
                return Ok(messages);
            }
        }
    }

    async fn try_receive(&self) -> BabataResult<Option<Vec<Message>>> {
        // Non-blocking poll (timeout=0), return immediately if there is no new DM message.
        let incoming = self.fetch_updates(0).await?;
        Ok(self.incoming_to_messages(incoming).await)
    }
}

#[derive(Debug)]
struct IncomingPrivateMessage {
    chat_id: i64,
    text: String,
}

fn extract_private_messages(
    updates: Vec<Update>,
    allowed_user_ids: &HashSet<i64>,
) -> (Option<i64>, Vec<IncomingPrivateMessage>) {
    let mut max_update_id = None;
    let mut messages = Vec::new();

    for update in updates {
        let update_id = i64::from(update.id.0);
        max_update_id = Some(max_update_id.map_or(update_id, |id: i64| id.max(update_id)));

        let message = match update.kind {
            UpdateKind::Message(message) => message,
            _ => continue,
        };

        // DM-only: ignore group/supergroup/channel updates.
        if !matches!(message.chat.kind, ChatKind::Private(_)) {
            continue;
        }

        let chat_id = message.chat.id.0;
        if !allowed_user_ids.contains(&chat_id) {
            continue;
        }

        let Some(text) = message.text() else {
            continue;
        };
        let text = text.trim();
        if text.is_empty() {
            continue;
        }

        messages.push(IncomingPrivateMessage {
            chat_id,
            text: text.to_string(),
        });
    }

    (max_update_id, messages)
}

fn extract_outgoing_texts(messages: &[Message]) -> Vec<String> {
    let mut outgoing = Vec::new();

    for message in messages {
        if let Message::AssistantResponse { content, .. } = message {
            let text = content
                .iter()
                .filter_map(|part| match part {
                    Content::Text { text } => Some(text.as_str()),
                    Content::ImageUrl { .. } | Content::ImageData { .. } => None,
                })
                .collect::<Vec<_>>()
                .join("\n");

            if !text.trim().is_empty() {
                outgoing.push(text);
            }
        }
    }

    outgoing
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_private_messages_filters_group_updates() {
        let updates = vec![
            serde_json::from_str::<Update>(
                r#"{
                    "message": {
                        "chat": {
                            "first_name": "Alice",
                            "id": 1001,
                            "type": "private",
                            "username": "alice"
                        },
                        "date": 1700000001,
                        "from": {
                            "first_name": "Alice",
                            "id": 1001,
                            "is_bot": false,
                            "language_code": "en",
                            "username": "alice"
                        },
                        "message_id": 101,
                        "text": "hello"
                    },
                    "update_id": 1
                }"#,
            )
            .expect("parse private update"),
            serde_json::from_str::<Update>(
                r#"{
                    "message": {
                        "chat": {
                            "id": -1002,
                            "title": "demo-group",
                            "type": "group"
                        },
                        "date": 1700000002,
                        "from": {
                            "first_name": "Bob",
                            "id": 1002,
                            "is_bot": false,
                            "username": "bob"
                        },
                        "message_id": 102,
                        "text": "group message"
                    },
                    "update_id": 2
                }"#,
            )
            .expect("parse group update"),
        ];

        let allowed_user_ids = HashSet::from([1001]);
        let (max_id, private_messages) = extract_private_messages(updates, &allowed_user_ids);
        assert_eq!(max_id, Some(2));
        assert_eq!(private_messages.len(), 1);
        assert_eq!(private_messages[0].chat_id, 1001);
        assert_eq!(private_messages[0].text, "hello");
    }

    #[test]
    fn extract_private_messages_filters_disallowed_users() {
        let updates = vec![
            serde_json::from_str::<Update>(
                r#"{
                    "message": {
                        "chat": {
                            "first_name": "Alice",
                            "id": 1001,
                            "type": "private",
                            "username": "alice"
                        },
                        "date": 1700000101,
                        "from": {
                            "first_name": "Alice",
                            "id": 1001,
                            "is_bot": false,
                            "language_code": "en",
                            "username": "alice"
                        },
                        "message_id": 201,
                        "text": "allow"
                    },
                    "update_id": 1
                }"#,
            )
            .expect("parse allowed update"),
            serde_json::from_str::<Update>(
                r#"{
                    "message": {
                        "chat": {
                            "first_name": "Carol",
                            "id": 2002,
                            "type": "private",
                            "username": "carol"
                        },
                        "date": 1700000102,
                        "from": {
                            "first_name": "Carol",
                            "id": 2002,
                            "is_bot": false,
                            "language_code": "en",
                            "username": "carol"
                        },
                        "message_id": 202,
                        "text": "deny"
                    },
                    "update_id": 2
                }"#,
            )
            .expect("parse disallowed update"),
        ];

        let allowed_user_ids = HashSet::from([1001]);
        let (max_id, private_messages) = extract_private_messages(updates, &allowed_user_ids);

        assert_eq!(max_id, Some(2));
        assert_eq!(private_messages.len(), 1);
        assert_eq!(private_messages[0].chat_id, 1001);
        assert_eq!(private_messages[0].text, "allow");
    }

    #[test]
    fn extract_outgoing_texts_keeps_only_assistant_text() {
        let messages = vec![
            Message::UserPrompt {
                content: vec![Content::Text {
                    text: "question".to_string(),
                }],
            },
            Message::AssistantResponse {
                content: vec![
                    Content::Text {
                        text: "line1".to_string(),
                    },
                    Content::Text {
                        text: "line2".to_string(),
                    },
                    Content::ImageUrl {
                        url: "https://example.com/image.png".to_string(),
                    },
                ],
                reasoning_content: None,
            },
        ];

        let outgoing = extract_outgoing_texts(&messages);
        assert_eq!(outgoing, vec!["line1\nline2".to_string()]);
    }
}
