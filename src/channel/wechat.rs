use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use log::warn;
use reqwest::Url;
use tokio::sync::{Mutex, oneshot};
use uuid::Uuid;
use wechatbot::{
    CdnClient, DownloadedMedia, IncomingMessage,
    protocol::{DEFAULT_BASE_URL, ILinkClient, build_text_message},
    types::CDNMedia as WechatCdnMedia,
};

use super::WechatChannelConfig;
use crate::{
    BabataResult,
    error::BabataError,
    message::{Content, MediaType},
    utils::channel_dir,
};

#[derive(Debug)]
pub struct WechatChannel {
    name: String,
    ilink_client: ILinkClient,
    cdn_client: CdnClient,
    bot_token: String,
    user_id: String,
    get_updates_buf: Mutex<Option<String>>,
    feedback_waiters: Mutex<HashMap<String, oneshot::Sender<Vec<Content>>>>,
}

impl WechatChannel {
    pub fn new(config: WechatChannelConfig) -> BabataResult<Self> {
        let WechatChannelConfig {
            name,
            bot_token,
            user_id,
        } = config;
        let get_updates_buf = Self::load_get_updates_buf(&name)?;
        Ok(Self {
            name,
            ilink_client: ILinkClient::new(),
            cdn_client: CdnClient::new(),
            bot_token,
            user_id,
            get_updates_buf: Mutex::new(get_updates_buf),
            feedback_waiters: Mutex::new(HashMap::new()),
        })
    }

    async fn fetch_updates(&self) -> BabataResult<Vec<IncomingMessage>> {
        let cursor = self
            .get_updates_buf
            .lock()
            .await
            .clone()
            .unwrap_or_default();
        let updates = self
            .ilink_client
            .get_updates(DEFAULT_BASE_URL, &self.bot_token, &cursor)
            .await
            .map_err(|err| {
                BabataError::channel(format!("Failed to fetch Wechat updates: {err}"))
            })?;

        if !updates.get_updates_buf.is_empty() {
            self.update_get_updates_buf(updates.get_updates_buf).await?;
        }

        Ok(updates
            .msgs
            .into_iter()
            .filter_map(|wire| IncomingMessage::from_wire(&wire))
            .collect())
    }

    async fn route_incoming(&self, incoming: Vec<IncomingMessage>) -> BabataResult<Vec<Content>> {
        let mut content = Vec::new();
        let one_hour_ago = SystemTime::now()
            .checked_sub(Duration::from_secs(3600))
            .unwrap_or(UNIX_EPOCH);

        for message in incoming {
            if message.user_id != self.user_id {
                continue;
            }

            let message_content = self.incoming_message_to_content(&message).await?;

            if let Some(hash) = extract_quote_hash(&message)
                && let Some(waiter) = self.feedback_waiters.lock().await.remove(&hash)
            {
                self.persist_latest_context_token(message.context_token())?;
                let _ = waiter.send(message_content);
                continue;
            }

            if message.timestamp < one_hour_ago {
                warn!(
                    "Ignoring Wechat message from {} older than 1 hour (timestamp: {:?})",
                    message.user_id, message.timestamp
                );
                continue;
            }

            self.persist_latest_context_token(message.context_token())?;
            content.extend(message_content);
        }

        Ok(content)
    }

    async fn update_get_updates_buf(&self, get_updates_buf: String) -> BabataResult<()> {
        {
            let mut current = self.get_updates_buf.lock().await;
            if current.as_deref() == Some(get_updates_buf.as_str()) {
                return Ok(());
            }
            *current = Some(get_updates_buf.clone());
        }
        write_text_file(&Self::get_updates_buf_path(&self.name)?, &get_updates_buf)
    }

    fn persist_latest_context_token(&self, context_token: &str) -> BabataResult<()> {
        write_text_file(&Self::latest_context_token_path(&self.name)?, context_token)
    }

    fn load_get_updates_buf(channel_name: &str) -> BabataResult<Option<String>> {
        read_optional_trimmed_text(&Self::get_updates_buf_path(channel_name)?)
    }

    fn get_updates_buf_path(channel_name: &str) -> BabataResult<PathBuf> {
        Ok(channel_dir(channel_name)?.join("get_updates_buf"))
    }

    fn load_latest_context_token(&self) -> BabataResult<Option<String>> {
        read_optional_trimmed_text(&Self::latest_context_token_path(&self.name)?)
    }

    fn latest_context_token_path(channel_name: &str) -> BabataResult<PathBuf> {
        Ok(channel_dir(channel_name)?.join("latest_context_token"))
    }

    async fn send_text_message(&self, context_token: &str, text: &str) -> BabataResult<()> {
        let msg = build_text_message(&self.user_id, context_token, text);
        self.ilink_client
            .send_message(DEFAULT_BASE_URL, &self.bot_token, &msg)
            .await
            .map_err(|err| BabataError::channel(format!("Failed to send Wechat message: {err}")))
    }

    async fn incoming_message_to_content(
        &self,
        message: &IncomingMessage,
    ) -> BabataResult<Vec<Content>> {
        let mut content = Vec::new();
        let mut text_parts = Vec::new();

        if let Some(quoted_text) = message
            .quoted
            .as_ref()
            .and_then(|quoted| quoted.text.as_deref())
            .and_then(non_empty)
        {
            text_parts.push(format!("> {quoted_text}"));
        }
        if let Some(text) = non_empty(&message.text) {
            text_parts.push(text.to_string());
        }

        for voice in &message.voices {
            if let Some(media) = voice.media.as_ref() {
                content.push(
                    self.attachment_content(
                        "audio",
                        Some("voice.silk"),
                        Some("audio/silk".to_string()),
                        media,
                        None,
                    )
                    .await?,
                );
            }
        }

        for image in &message.images {
            if let Some(media) = image.media.as_ref() {
                content.push(
                    self.attachment_content(
                        "image",
                        Some("image"),
                        image.url.as_deref().and_then(infer_mime_type_from_url),
                        media,
                        image.aes_key.as_deref().and_then(non_empty),
                    )
                    .await?,
                );
            }
        }

        for file in &message.files {
            if let Some(media) = file.media.as_ref() {
                content.push(
                    self.attachment_content(
                        "file",
                        file.file_name.as_deref().and_then(non_empty),
                        file.file_name
                            .as_deref()
                            .and_then(infer_mime_type_from_name),
                        media,
                        None,
                    )
                    .await?,
                );
            }
        }

        for video in &message.videos {
            if let Some(media) = video.media.as_ref() {
                content.push(
                    self.attachment_content(
                        "video",
                        Some("video.mp4"),
                        Some("video/mp4".to_string()),
                        media,
                        None,
                    )
                    .await?,
                );
            }
        }

        let text = text_parts.join("\n");
        if !text.is_empty() {
            content.insert(0, Content::Text { text });
        }

        Ok(content)
    }

    async fn attachment_content(
        &self,
        kind_name: &'static str,
        file_name: Option<&str>,
        mime_type: Option<String>,
        media: &WechatCdnMedia,
        aes_key_override: Option<&str>,
    ) -> BabataResult<Content> {
        let downloaded = self
            .download_cdn_attachment(media, aes_key_override, kind_name, file_name)
            .await?;
        self.content_from_downloaded_attachment(kind_name, file_name, mime_type, downloaded)
    }

    fn content_from_downloaded_attachment(
        &self,
        kind_name: &str,
        file_name: Option<&str>,
        mime_type: Option<String>,
        downloaded: DownloadedMedia,
    ) -> BabataResult<Content> {
        let file_name = file_name.or(downloaded.file_name.as_deref());
        let mime_type = mime_type
            .or_else(|| {
                downloaded
                    .file_name
                    .as_deref()
                    .and_then(infer_mime_type_from_name)
            })
            .unwrap_or_else(|| "application/octet-stream".to_string());

        if kind_name == "image"
            && let Some(media_type) = MediaType::from_mime(&mime_type)
                .or_else(|| detect_image_media_type(&downloaded.data))
        {
            return Ok(Content::ImageData {
                data: STANDARD.encode(&downloaded.data),
                media_type,
            });
        }

        self.persisted_attachment_content(kind_name, file_name, &downloaded.data, &mime_type)
    }

    fn persisted_attachment_content(
        &self,
        kind_name: &str,
        file_name: Option<&str>,
        data: &[u8],
        mime_type: &str,
    ) -> BabataResult<Content> {
        let path = self.persist_attachment(file_name, kind_name, data)?;
        Ok(Content::Text {
            text: format!(
                "kind: {}, local_path: {}, mime_type: {}",
                kind_name,
                path.display(),
                mime_type
            ),
        })
    }

    async fn download_cdn_attachment(
        &self,
        media: &WechatCdnMedia,
        aes_key_override: Option<&str>,
        kind_name: &str,
        file_name: Option<&str>,
    ) -> BabataResult<DownloadedMedia> {
        let data = self
            .cdn_client
            .download(media, aes_key_override)
            .await
            .map_err(|err| {
                BabataError::channel(format!(
                    "Failed to download Wechat attachment '{:?}' from CDN: {:?}",
                    media, err
                ))
            })?;
        let format = match kind_name {
            "voice" => Some("silk".to_string()),
            _ => None,
        };
        Ok(DownloadedMedia {
            data,
            media_type: kind_name.to_string(),
            file_name: file_name.map(ToString::to_string),
            format,
        })
    }

    fn persist_attachment(
        &self,
        file_name: Option<&str>,
        kind_name: &str,
        data: &[u8],
    ) -> BabataResult<PathBuf> {
        let dir = Self::media_dir(&self.name)?;
        std::fs::create_dir_all(&dir)?;

        let file_name = sanitize_file_name(file_name.unwrap_or(kind_name));
        let path = dir.join(format!("{}_{}", Uuid::new_v4(), file_name));
        std::fs::write(&path, data)?;
        Ok(path)
    }

    fn media_dir(channel_name: &str) -> BabataResult<PathBuf> {
        Ok(channel_dir(channel_name)?.join("media"))
    }
}

fn render_feedback_text(content: &[Content]) -> BabataResult<String> {
    let text = content
        .iter()
        .map(|item| match item {
            Content::Text { text } => Ok(text.trim().to_string()),
            Content::ImageUrl { url } => Ok(url.clone()),
            Content::ImageData { .. } | Content::AudioData { .. } => Err(BabataError::channel(
                "Wechat feedback only supports text or image URLs for outbound prompts",
            )),
        })
        .collect::<BabataResult<Vec<_>>>()?
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");

    if text.is_empty() {
        return Err(BabataError::channel(
            "Wechat feedback requires non-empty outbound content",
        ));
    }
    Ok(text)
}

#[async_trait::async_trait]
impl super::Channel for WechatChannel {
    async fn try_receive(&self) -> BabataResult<Vec<Content>> {
        let incoming = self.fetch_updates().await?;
        self.route_incoming(incoming).await
    }

    async fn feedback(&self, content: Vec<Content>) -> BabataResult<Vec<Content>> {
        let text = render_feedback_text(&content)?;
        let context_token = self.load_latest_context_token()?.ok_or_else(|| {
            BabataError::channel(
                "Wechat context_token not found; no messages have been received yet".to_string(),
            )
        })?;

        self.send_text_message(&context_token, &text).await?;

        let (sender, receiver) = oneshot::channel();
        self.feedback_waiters
            .lock()
            .await
            .insert(hash_feedback_key(&text), sender);

        receiver.await.map_err(|_| {
            BabataError::channel("Wechat feedback waiter was dropped before reply arrived")
        })
    }
}

fn extract_quote_hash(message: &IncomingMessage) -> Option<String> {
    message
        .quoted
        .as_ref()
        .and_then(|quoted| quoted.text.as_deref())
        .and_then(non_empty)
        .map(hash_feedback_key)
}

fn hash_feedback_key(content: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

fn read_optional_trimmed_text(path: &Path) -> BabataResult<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)?;
    Ok(non_empty(&content).map(ToString::to_string))
}

fn write_text_file(path: &Path, content: &str) -> BabataResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(())
}

fn infer_mime_type_from_name(name: &str) -> Option<String> {
    let name = non_empty(name)?;
    Some(
        mime_guess::from_path(name)
            .first_or_octet_stream()
            .to_string(),
    )
}

fn infer_mime_type_from_url(url: &str) -> Option<String> {
    let url = non_empty(url)?;
    let parsed = Url::parse(url).ok()?;
    infer_mime_type_from_name(parsed.path())
}

fn sanitize_file_name(file_name: &str) -> String {
    let sanitized = file_name
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => ch,
        })
        .collect::<String>();

    let sanitized = sanitized.trim();
    if sanitized.is_empty() {
        "file".to_string()
    } else {
        sanitized.to_string()
    }
}

fn detect_image_media_type(data: &[u8]) -> Option<MediaType> {
    if data.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some(MediaType::ImagePng);
    }
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(MediaType::ImageJpeg);
    }
    if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        return Some(MediaType::ImageGif);
    }
    if data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        return Some(MediaType::ImageWebp);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use wechatbot::types::{CDNMedia, ImageContent, ImageItem as WechatImageItem};

    #[test]
    fn image_attachment_uses_cdn_media() {
        let image = WechatImageItem {
            media: Some(CDNMedia {
                encrypt_query_param: "encrypted-param".to_string(),
                aes_key: "ABEiM0RVZneImaq7zN3u/w==".to_string(),
                encrypt_type: None,
                full_url: None,
            }),
            thumb_media: None,
            aeskey: Some("00112233445566778899aabbccddeeff".to_string()),
            url: Some("https://example.com/path/image.png".to_string()),
            mid_size: Some(128),
            thumb_width: None,
            thumb_height: None,
        };

        assert_eq!(image.mid_size.map(|size| size as u64), Some(128));
        assert_eq!(
            image
                .url
                .as_deref()
                .and_then(infer_mime_type_from_url)
                .as_deref(),
            Some("image/png")
        );
        assert_eq!(
            image.aeskey.as_deref().and_then(non_empty),
            Some("00112233445566778899aabbccddeeff")
        );
    }

    #[test]
    fn sanitize_file_name_replaces_path_separators() {
        assert_eq!(
            sanitize_file_name(r#"a/b\c:d*e?f"g<h>i|.txt"#),
            "a_b_c_d_e_f_g_h_i_.txt"
        );
    }

    #[test]
    fn detect_image_media_type_reads_common_magic_bytes() {
        assert_eq!(
            detect_image_media_type(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]),
            Some(MediaType::ImagePng)
        );
        assert_eq!(
            detect_image_media_type(&[0xFF, 0xD8, 0xFF, 0xE0]),
            Some(MediaType::ImageJpeg)
        );
        assert_eq!(
            detect_image_media_type(b"GIF89a123"),
            Some(MediaType::ImageGif)
        );
        assert_eq!(
            detect_image_media_type(b"RIFFxxxxWEBPmore"),
            Some(MediaType::ImageWebp)
        );
    }

    #[test]
    fn infer_mime_type_from_url_reads_path_suffix() {
        assert_eq!(
            infer_mime_type_from_url("https://example.com/assets/demo.webp").as_deref(),
            Some("image/webp")
        );
    }

    #[test]
    fn incoming_message_image_metadata_uses_high_level_fields() {
        let image = ImageContent {
            media: Some(CDNMedia {
                encrypt_query_param: "encrypted-param".to_string(),
                aes_key: "key".to_string(),
                encrypt_type: None,
                full_url: None,
            }),
            thumb_media: None,
            aes_key: Some("00112233445566778899aabbccddeeff".to_string()),
            url: Some("https://example.com/path/image.png".to_string()),
            width: Some(100),
            height: Some(200),
        };

        assert_eq!(
            image
                .url
                .as_deref()
                .and_then(infer_mime_type_from_url)
                .as_deref(),
            Some("image/png")
        );
        assert_eq!(
            image.aes_key.as_deref().and_then(non_empty),
            Some("00112233445566778899aabbccddeeff")
        );
    }
}
