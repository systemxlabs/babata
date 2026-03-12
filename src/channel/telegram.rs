use std::collections::HashMap;

use log::warn;
use reqwest::{Client, StatusCode};
use teloxide::{
    Bot,
    payloads::GetUpdatesSetters,
    prelude::{Request, Requester},
    types::{ChatId, ChatKind, Document, Message as TelegramMessage, Update, UpdateKind},
};
use tokio::sync::{Mutex, oneshot};

use crate::{
    BabataResult,
    config::{ChannelConfig, Config, TelegramChannelConfig},
    error::BabataError,
    message::{Content, MediaType},
};

const DEFAULT_POLLING_TIMEOUT_SECS: u64 = 30;

#[derive(Debug)]
pub struct TelegramChannel {
    bot: Bot,
    http_client: Client,
    // Telegram update cursor to avoid reprocessing already consumed updates.
    last_update_id: Mutex<Option<i64>>,
    // Waiters for replies to outbound feedback prompts, keyed by sent message id.
    feedback_waiters: Mutex<HashMap<i32, oneshot::Sender<Vec<Content>>>>,
    // Allowed DM user id; messages from others are ignored.
    user_id: i64,
}

impl TelegramChannel {
    pub fn new(config: TelegramChannelConfig) -> Self {
        let TelegramChannelConfig {
            bot_token,
            last_update_id,
            user_id,
        } = config;

        Self {
            bot: Bot::new(bot_token),
            http_client: Client::new(),
            last_update_id: Mutex::new(last_update_id),
            feedback_waiters: Mutex::new(HashMap::new()),
            user_id,
        }
    }

    async fn fetch_updates(&self) -> BabataResult<Vec<IncomingPrivateMessage>> {
        let offset = *self.last_update_id.lock().await;
        let mut request = self
            .bot
            .get_updates()
            .timeout(DEFAULT_POLLING_TIMEOUT_SECS as u32);

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
        let (max_update_id, messages) = extract_private_messages(updates, self.user_id);

        if let Some(max_update_id) = max_update_id {
            self.update_last_update_id(max_update_id).await?;
        }

        Ok(messages)
    }

    async fn route_incoming(&self, incoming: Vec<IncomingPrivateMessage>) -> Vec<Content> {
        let mut content = Vec::new();

        for message in incoming {
            if message.chat_id != self.user_id {
                continue;
            }

            let Some(message_content) = self.incoming_message_to_content(&message).await else {
                continue;
            };

            if let Some(reply_to_message_id) = message.reply_to_message_id {
                let waiter = self
                    .feedback_waiters
                    .lock()
                    .await
                    .remove(&reply_to_message_id);
                if let Some(waiter) = waiter {
                    let _ = waiter.send(message_content);
                }
                continue;
            }

            content.extend(message_content);
        }

        content
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

    async fn incoming_message_to_content(
        &self,
        message: &IncomingPrivateMessage,
    ) -> Option<Vec<Content>> {
        let mut content = Vec::new();

        if let Some(text) = &message.text {
            content.push(Content::Text { text: text.clone() });
        }

        if let Some(image_file_id) = &message.image_file_id {
            let media_type = message
                .image_media_type
                .clone()
                .unwrap_or_else(|| "image/jpeg".to_string());
            match self
                .download_file_as_base64(image_file_id, &media_type)
                .await
            {
                Ok(data) => match MediaType::from_mime(&media_type) {
                    Some(media_type) => content.push(Content::ImageData { data, media_type }),
                    None => warn!(
                        "Unsupported Telegram image media type '{}'; skipping image content.",
                        media_type
                    ),
                },
                Err(err) => {
                    warn!(
                        "Failed to process Telegram image file '{}': {}. Continuing without image.",
                        image_file_id, err
                    );
                }
            }
        }

        if let Some(audio_file_id) = &message.audio_file_id {
            let media_type = message
                .audio_media_type
                .clone()
                .unwrap_or_else(|| "audio/ogg".to_string());
            match self
                .download_file_as_base64(audio_file_id, &media_type)
                .await
            {
                Ok(data) => match MediaType::from_mime(&media_type) {
                    Some(media_type) => content.push(Content::AudioData { data, media_type }),
                    None => warn!(
                        "Unsupported Telegram audio media type '{}'; skipping audio content.",
                        media_type
                    ),
                },
                Err(err) => {
                    warn!(
                        "Failed to process Telegram audio file '{}': {}. Continuing without audio.",
                        audio_file_id, err
                    );
                }
            }
        }

        (!content.is_empty()).then_some(content)
    }

    async fn download_file_as_base64(
        &self,
        file_id: &str,
        media_type: &str,
    ) -> BabataResult<String> {
        let file = self
            .bot
            .get_file(file_id.to_string())
            .send()
            .await
            .map_err(|err| {
                BabataError::internal(format!(
                    "Failed to call Telegram getFile for '{}': {err}",
                    file_id
                ))
            })?;

        let path = file.path.trim_start_matches('/');
        let file_url = format!(
            "https://api.telegram.org/file/bot{}/{}",
            self.bot.token(),
            path
        );
        let response = self
            .http_client
            .get(&file_url)
            .send()
            .await
            .map_err(|err| {
                BabataError::internal(format!(
                    "Failed to download Telegram file '{}' ({media_type}): {err}",
                    file_id
                ))
            })?;

        let status = response.status();
        if status != StatusCode::OK {
            let body = response.text().await.unwrap_or_default();
            return Err(BabataError::internal(format!(
                "Telegram file download failed for '{}' with status {}: {}",
                file_id, status, body
            )));
        }

        let bytes = response.bytes().await.map_err(|err| {
            BabataError::internal(format!(
                "Failed to read Telegram file bytes for '{}': {err}",
                file_id
            ))
        })?;

        use base64::Engine as _;
        Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
    }
}

#[async_trait::async_trait]
impl super::Channel for TelegramChannel {
    fn name() -> &'static str {
        "Telegram"
    }

    async fn try_receive(&self) -> BabataResult<Vec<Content>> {
        let incoming = self.fetch_updates().await?;
        Ok(self.route_incoming(incoming).await)
    }

    async fn feedback(&self, content: Vec<Content>) -> BabataResult<Vec<Content>> {
        let text = render_feedback_text(&content)?;
        let sent_message = self
            .bot
            .send_message(ChatId(self.user_id), text)
            .send()
            .await
            .map_err(|err| {
                BabataError::internal(format!("Failed to send Telegram feedback message: {err}"))
            })?;

        let (sender, receiver) = oneshot::channel();
        self.feedback_waiters
            .lock()
            .await
            .insert(sent_message.id.0, sender);

        receiver.await.map_err(|_| {
            BabataError::internal("Telegram feedback waiter was dropped before reply arrived")
        })
    }
}

#[derive(Debug)]
struct IncomingPrivateMessage {
    chat_id: i64,
    reply_to_message_id: Option<i32>,
    text: Option<String>,
    image_file_id: Option<String>,
    image_media_type: Option<String>,
    audio_file_id: Option<String>,
    audio_media_type: Option<String>,
}

fn extract_private_messages(
    updates: Vec<Update>,
    user_id: i64,
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
        if chat_id != user_id {
            continue;
        }

        let text = message
            .text()
            .or_else(|| message.caption())
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(ToString::to_string);

        let (image_file_id, image_media_type) = extract_incoming_image(&message);
        let (audio_file_id, audio_media_type) = extract_incoming_audio(&message);

        if text.is_none() && image_file_id.is_none() && audio_file_id.is_none() {
            continue;
        }

        messages.push(IncomingPrivateMessage {
            chat_id,
            reply_to_message_id: message.reply_to_message().map(|reply| reply.id.0),
            text,
            image_file_id,
            image_media_type,
            audio_file_id,
            audio_media_type,
        });
    }

    (max_update_id, messages)
}

fn extract_incoming_image(message: &TelegramMessage) -> (Option<String>, Option<String>) {
    if let Some(photos) = message.photo() {
        // Telegram returns multiple sizes for a single photo; keep the largest one.
        if let Some(photo) = photos.last() {
            return (Some(photo.file.id.clone()), Some("image/jpeg".to_string()));
        }
    }

    if let Some(document) = message.document() {
        return extract_image_document(document);
    }

    (None, None)
}

fn extract_image_document(document: &Document) -> (Option<String>, Option<String>) {
    let media_type = document
        .mime_type
        .as_ref()
        .map(ToString::to_string)
        .filter(|mime| mime.starts_with("image/"));

    if media_type.is_none() {
        return (None, None);
    }

    (Some(document.file.id.clone()), media_type)
}

fn extract_incoming_audio(message: &TelegramMessage) -> (Option<String>, Option<String>) {
    if let Some(audio) = message.audio() {
        let media_type = audio
            .mime_type
            .as_ref()
            .map(ToString::to_string)
            .filter(|mime| mime.starts_with("audio/"))
            .unwrap_or_else(|| "audio/mpeg".to_string());
        return (Some(audio.file.id.clone()), Some(media_type));
    }

    if let Some(voice) = message.voice() {
        // Telegram voice messages are always encoded as OGG/OPUS.
        return (Some(voice.file.id.clone()), Some("audio/ogg".to_string()));
    }

    if let Some(document) = message.document() {
        return extract_audio_document(document);
    }

    (None, None)
}

fn extract_audio_document(document: &Document) -> (Option<String>, Option<String>) {
    let media_type = document
        .mime_type
        .as_ref()
        .map(ToString::to_string)
        .filter(|mime| mime.starts_with("audio/"));

    if media_type.is_none() {
        return (None, None);
    }

    (Some(document.file.id.clone()), media_type)
}

fn render_feedback_text(content: &[Content]) -> BabataResult<String> {
    let text = content
        .iter()
        .map(|item| match item {
            Content::Text { text } => Ok(text.trim().to_string()),
            Content::ImageUrl { url } => Ok(url.clone()),
            Content::ImageData { .. } | Content::AudioData { .. } => Err(BabataError::internal(
                "Telegram feedback only supports text or image URLs for outbound prompts",
            )),
        })
        .collect::<BabataResult<Vec<_>>>()?
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");

    if text.is_empty() {
        return Err(BabataError::internal(
            "Telegram feedback requires non-empty outbound content",
        ));
    }

    Ok(text)
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

        let (max_id, private_messages) = extract_private_messages(updates, 1001);
        assert_eq!(max_id, Some(2));
        assert_eq!(private_messages.len(), 1);
        assert_eq!(private_messages[0].chat_id, 1001);
        assert_eq!(private_messages[0].text, Some("hello".to_string()));
        assert!(private_messages[0].image_file_id.is_none());
        assert!(private_messages[0].audio_file_id.is_none());
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

        let (max_id, private_messages) = extract_private_messages(updates, 1001);

        assert_eq!(max_id, Some(2));
        assert_eq!(private_messages.len(), 1);
        assert_eq!(private_messages[0].chat_id, 1001);
        assert_eq!(private_messages[0].text, Some("allow".to_string()));
        assert!(private_messages[0].image_file_id.is_none());
        assert!(private_messages[0].audio_file_id.is_none());
    }

    #[test]
    fn extract_private_messages_supports_photo_message() {
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
                    "date": 1700000201,
                    "from": {
                        "first_name": "Alice",
                        "id": 1001,
                        "is_bot": false,
                        "language_code": "en",
                        "username": "alice"
                    },
                    "message_id": 301,
                    "photo": [
                        {
                            "file_id": "photo-small",
                            "file_unique_id": "small-1",
                            "width": 90,
                            "height": 90,
                            "file_size": 1024
                        },
                        {
                            "file_id": "photo-large",
                            "file_unique_id": "large-1",
                            "width": 640,
                            "height": 640,
                            "file_size": 40960
                        }
                    ]
                },
                "update_id": 3
            }"#,
            )
            .expect("parse photo update"),
        ];

        let (max_id, private_messages) = extract_private_messages(updates, 1001);

        assert_eq!(max_id, Some(3));
        assert_eq!(private_messages.len(), 1);
        assert_eq!(private_messages[0].chat_id, 1001);
        assert!(private_messages[0].text.is_none());
        assert_eq!(
            private_messages[0].image_file_id,
            Some("photo-large".to_string())
        );
        assert_eq!(
            private_messages[0].image_media_type,
            Some("image/jpeg".to_string())
        );
        assert!(private_messages[0].audio_file_id.is_none());
    }

    #[test]
    fn extract_private_messages_supports_image_document_message() {
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
                    "date": 1700000202,
                    "from": {
                        "first_name": "Alice",
                        "id": 1001,
                        "is_bot": false,
                        "language_code": "en",
                        "username": "alice"
                    },
                    "message_id": 302,
                    "document": {
                        "file_id": "doc-image-1",
                        "file_unique_id": "doc-unique-1",
                        "file_size": 2048,
                        "file_name": "image.png",
                        "mime_type": "image/png"
                    }
                },
                "update_id": 4
            }"#,
            )
            .expect("parse image document update"),
        ];

        let (max_id, private_messages) = extract_private_messages(updates, 1001);

        assert_eq!(max_id, Some(4));
        assert_eq!(private_messages.len(), 1);
        assert_eq!(private_messages[0].chat_id, 1001);
        assert!(private_messages[0].text.is_none());
        assert_eq!(
            private_messages[0].image_file_id,
            Some("doc-image-1".to_string())
        );
        assert_eq!(
            private_messages[0].image_media_type,
            Some("image/png".to_string())
        );
        assert!(private_messages[0].audio_file_id.is_none());
    }

    #[test]
    fn extract_private_messages_supports_voice_message() {
        let update = serde_json::from_str::<Update>(
            r#"{
                "message": {
                    "chat": {
                        "first_name": "Alice",
                        "id": 1001,
                        "type": "private",
                        "username": "alice"
                    },
                    "date": 1700000203,
                    "from": {
                        "first_name": "Alice",
                        "id": 1001,
                        "is_bot": false,
                        "language_code": "en",
                        "username": "alice"
                    },
                    "message_id": 303,
                    "voice": {
                        "duration": 2,
                        "file_id": "voice-1",
                        "file_unique_id": "voice-unique-1",
                        "mime_type": "audio/ogg",
                        "file_size": 4096
                    }
                },
                "update_id": 5
            }"#,
        )
        .expect("parse voice update");

        let UpdateKind::Message(parsed_message) = &update.kind else {
            panic!("expected message update kind");
        };
        assert!(
            parsed_message.voice().is_some(),
            "expected voice payload to be parsed"
        );

        let updates = vec![update];

        let (max_id, private_messages) = extract_private_messages(updates, 1001);

        assert_eq!(max_id, Some(5));
        assert_eq!(private_messages.len(), 1);
        assert_eq!(private_messages[0].chat_id, 1001);
        assert!(private_messages[0].text.is_none());
        assert!(private_messages[0].image_file_id.is_none());
        assert_eq!(
            private_messages[0].audio_file_id,
            Some("voice-1".to_string())
        );
        assert_eq!(
            private_messages[0].audio_media_type,
            Some("audio/ogg".to_string())
        );
    }

    #[test]
    fn extract_private_messages_supports_audio_document_message() {
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
                    "date": 1700000204,
                    "from": {
                        "first_name": "Alice",
                        "id": 1001,
                        "is_bot": false,
                        "language_code": "en",
                        "username": "alice"
                    },
                    "message_id": 304,
                    "document": {
                        "file_id": "doc-audio-1",
                        "file_unique_id": "doc-audio-unique-1",
                        "file_size": 4096,
                        "file_name": "audio.ogg",
                        "mime_type": "audio/ogg"
                    }
                },
                "update_id": 6
            }"#,
            )
            .expect("parse audio document update"),
        ];

        let (max_id, private_messages) = extract_private_messages(updates, 1001);

        assert_eq!(max_id, Some(6));
        assert_eq!(private_messages.len(), 1);
        assert_eq!(private_messages[0].chat_id, 1001);
        assert!(private_messages[0].text.is_none());
        assert!(private_messages[0].image_file_id.is_none());
        assert_eq!(
            private_messages[0].audio_file_id,
            Some("doc-audio-1".to_string())
        );
        assert_eq!(
            private_messages[0].audio_media_type,
            Some("audio/ogg".to_string())
        );
    }
}
