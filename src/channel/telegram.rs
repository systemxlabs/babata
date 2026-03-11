use std::collections::HashSet;

use log::warn;
use reqwest::{Client, StatusCode};
use teloxide::{
    Bot,
    payloads::GetUpdatesSetters,
    prelude::{Request, Requester},
    types::{ChatKind, Document, Message as TelegramMessage, Update, UpdateKind},
};
use tokio::sync::Mutex;

use crate::{
    BabataResult,
    config::{ChannelConfig, Config},
    error::BabataError,
    message::{Content, MediaType, Message},
};

#[derive(Debug)]
pub struct TelegramChannel {
    bot: Bot,
    http_client: Client,
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
            http_client: Client::new(),
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

    async fn incoming_to_messages(
        &self,
        incoming: Vec<IncomingPrivateMessage>,
    ) -> Option<Vec<Message>> {
        if incoming.is_empty() {
            return None;
        }

        let mut messages = Vec::new();
        for message in incoming {
            if !self.allowed_user_ids.contains(&message.chat_id) {
                continue;
            }

            let mut content = Vec::new();
            if let Some(text) = message.text {
                content.push(Content::Text { text });
            }

            if let Some(image_file_id) = message.image_file_id {
                let media_type = message
                    .image_media_type
                    .unwrap_or_else(|| "image/jpeg".to_string());
                match self
                    .download_file_as_base64(&image_file_id, &media_type)
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

            if let Some(audio_file_id) = message.audio_file_id {
                let media_type = message
                    .audio_media_type
                    .unwrap_or_else(|| "audio/ogg".to_string());
                match self
                    .download_file_as_base64(&audio_file_id, &media_type)
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

            if content.is_empty() {
                continue;
            }

            messages.push(Message::UserPrompt { content });
        }

        if messages.is_empty() {
            return None;
        }

        Some(messages)
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
    text: Option<String>,
    image_file_id: Option<String>,
    image_media_type: Option<String>,
    audio_file_id: Option<String>,
    audio_media_type: Option<String>,
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

        let allowed_user_ids = HashSet::from([1001]);
        let (max_id, private_messages) = extract_private_messages(updates, &allowed_user_ids);

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

        let allowed_user_ids = HashSet::from([1001]);
        let (max_id, private_messages) = extract_private_messages(updates, &allowed_user_ids);

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

        let allowed_user_ids = HashSet::from([1001]);
        let (max_id, private_messages) = extract_private_messages(updates, &allowed_user_ids);

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

        let allowed_user_ids = HashSet::from([1001]);
        let (max_id, private_messages) = extract_private_messages(updates, &allowed_user_ids);

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

        let allowed_user_ids = HashSet::from([1001]);
        let (max_id, private_messages) = extract_private_messages(updates, &allowed_user_ids);

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
