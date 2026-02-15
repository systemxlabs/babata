use std::collections::HashSet;

use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::Mutex;

use crate::{
    BabataResult,
    error::BabataError,
    message::{Content, Message},
};

#[derive(Debug)]
pub struct TelegramChannel {
    client: Client,
    bot_token: String,
    base_url: String,
    // Long-poll timeout used by blocking receive().
    polling_timeout_secs: u64,
    // Telegram update cursor to avoid reprocessing already consumed updates.
    last_update_id: Mutex<Option<i64>>,
    // The current DM chat to reply to; group chats are intentionally unsupported.
    active_private_chat_id: Mutex<Option<i64>>,
    // Allowed DM user ids; messages from others are ignored.
    allowed_user_ids: HashSet<i64>,
}

impl TelegramChannel {
    pub fn new(bot_token: &str) -> Self {
        Self {
            client: Client::new(),
            bot_token: bot_token.to_string(),
            base_url: "https://api.telegram.org".to_string(),
            polling_timeout_secs: 30,
            last_update_id: Mutex::new(None),
            active_private_chat_id: Mutex::new(None),
            allowed_user_ids: HashSet::new(),
        }
    }

    pub fn with_base_url(mut self, base_url: &str) -> Self {
        self.base_url = base_url.trim_end_matches('/').to_string();
        self
    }

    pub fn with_polling_timeout_secs(mut self, polling_timeout_secs: u64) -> Self {
        self.polling_timeout_secs = polling_timeout_secs;
        self
    }

    pub fn with_allowed_user_ids(mut self, allowed_user_ids: Vec<i64>) -> Self {
        self.allowed_user_ids = allowed_user_ids.into_iter().collect();
        self
    }

    fn endpoint(&self, method: &str) -> String {
        format!("{}/bot{}/{}", self.base_url, self.bot_token, method)
    }

    async fn fetch_updates(&self, timeout_secs: u64) -> BabataResult<Vec<IncomingPrivateMessage>> {
        let offset = *self.last_update_id.lock().await;

        let mut body = json!({
            "timeout": timeout_secs
        });
        if let Some(offset) = offset {
            body["offset"] = json!(offset + 1);
        }

        let response = self
            .client
            .post(self.endpoint("getUpdates"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|err| {
                BabataError::internal(format!("Failed to call Telegram getUpdates: {err}"))
            })?;

        let status = response.status();
        if status != StatusCode::OK {
            let body = response.text().await.unwrap_or_default();
            return Err(BabataError::internal(format!(
                "Telegram getUpdates returned status {status}: {body}"
            )));
        }

        let payload: TelegramResponse<Vec<TelegramUpdate>> =
            response.json().await.map_err(|err| {
                BabataError::internal(format!(
                    "Failed to parse Telegram getUpdates response: {err}"
                ))
            })?;

        if !payload.ok {
            return Err(BabataError::internal(format!(
                "Telegram getUpdates failed: {}",
                payload
                    .description
                    .unwrap_or_else(|| "Unknown error".to_string())
            )));
        }

        let updates = payload.result.unwrap_or_default();
        // Keep only DM messages and return the max update_id for cursor advancing.
        let (max_update_id, messages) = extract_private_messages(updates, &self.allowed_user_ids);

        if let Some(max_update_id) = max_update_id {
            *self.last_update_id.lock().await = Some(max_update_id);
        }

        Ok(messages)
    }

    async fn send_text(&self, chat_id: i64, text: &str) -> BabataResult<()> {
        let body = json!({
            "chat_id": chat_id,
            "text": text
        });

        let response = self
            .client
            .post(self.endpoint("sendMessage"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|err| {
                BabataError::internal(format!("Failed to call Telegram sendMessage: {err}"))
            })?;

        let status = response.status();
        if status != StatusCode::OK {
            let body = response.text().await.unwrap_or_default();
            return Err(BabataError::internal(format!(
                "Telegram sendMessage returned status {status}: {body}"
            )));
        }

        let payload: TelegramResponse<serde_json::Value> =
            response.json().await.map_err(|err| {
                BabataError::internal(format!(
                    "Failed to parse Telegram sendMessage response: {err}"
                ))
            })?;

        if !payload.ok {
            return Err(BabataError::internal(format!(
                "Telegram sendMessage failed: {}",
                payload
                    .description
                    .unwrap_or_else(|| "Unknown error".to_string())
            )));
        }

        Ok(())
    }

    async fn incoming_to_messages(
        &self,
        incoming: Vec<IncomingPrivateMessage>,
    ) -> Option<Vec<Message>> {
        if incoming.is_empty() {
            return None;
        }

        // Pin replies to the first DM chat in this batch.
        let active_chat_id = incoming[0].chat_id;
        *self.active_private_chat_id.lock().await = Some(active_chat_id);

        let messages = incoming
            .into_iter()
            .filter(|message| message.chat_id == active_chat_id)
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
        // Outbound messages are sent only to the active DM chat.
        let chat_id = (*self.active_private_chat_id.lock().await).ok_or_else(|| {
            BabataError::internal("No active private Telegram chat available for sending")
        })?;

        let outgoing = extract_outgoing_texts(messages);
        for text in outgoing {
            self.send_text(chat_id, &text).await?;
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

#[derive(Debug, Deserialize)]
struct TelegramResponse<T> {
    ok: bool,
    result: Option<T>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TelegramUpdate {
    update_id: i64,
    message: Option<TelegramMessage>,
}

#[derive(Debug, Deserialize)]
struct TelegramMessage {
    chat: TelegramChat,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TelegramChat {
    id: i64,
    #[serde(rename = "type")]
    chat_type: String,
}

fn extract_private_messages(
    updates: Vec<TelegramUpdate>,
    allowed_user_ids: &HashSet<i64>,
) -> (Option<i64>, Vec<IncomingPrivateMessage>) {
    let mut max_update_id = None;
    let mut messages = Vec::new();

    for update in updates {
        max_update_id =
            Some(max_update_id.map_or(update.update_id, |id: i64| id.max(update.update_id)));

        let Some(message) = update.message else {
            continue;
        };
        // DM-only: ignore group/supergroup/channel updates.
        if message.chat.chat_type != "private" {
            continue;
        }
        if !allowed_user_ids.contains(&message.chat.id) {
            continue;
        }
        let Some(text) = message.text else {
            continue;
        };
        let text = text.trim();
        if text.is_empty() {
            continue;
        }

        messages.push(IncomingPrivateMessage {
            chat_id: message.chat.id,
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
        let updates_json = serde_json::json!([
            {
                "update_id": 1,
                "message": {
                    "chat": { "id": 1001, "type": "private" },
                    "text": "hello"
                }
            },
            {
                "update_id": 2,
                "message": {
                    "chat": { "id": -1002, "type": "group" },
                    "text": "group message"
                }
            }
        ]);

        let updates: Vec<TelegramUpdate> =
            serde_json::from_value(updates_json).expect("parse updates json");

        let allowed_user_ids = HashSet::from([1001]);
        let (max_id, private_messages) = extract_private_messages(updates, &allowed_user_ids);
        assert_eq!(max_id, Some(2));
        assert_eq!(private_messages.len(), 1);
        assert_eq!(private_messages[0].chat_id, 1001);
        assert_eq!(private_messages[0].text, "hello");
    }

    #[test]
    fn extract_private_messages_filters_disallowed_users() {
        let updates_json = serde_json::json!([
            {
                "update_id": 1,
                "message": {
                    "chat": { "id": 1001, "type": "private" },
                    "text": "allow"
                }
            },
            {
                "update_id": 2,
                "message": {
                    "chat": { "id": 2002, "type": "private" },
                    "text": "deny"
                }
            }
        ]);

        let updates: Vec<TelegramUpdate> =
            serde_json::from_value(updates_json).expect("parse updates json");

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
