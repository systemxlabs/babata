use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::Duration,
};

use aes::Aes128;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use block_padding::Pkcs7;
use cipher::{BlockDecryptMut as _, KeyInit};
use log::warn;
use reqwest::{
    Client, StatusCode, Url,
    header::{AUTHORIZATION, HeaderMap, HeaderName, HeaderValue},
};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    BabataResult,
    config::WechatChannelConfig,
    error::BabataError,
    message::{Content, MediaType},
    utils::babata_dir,
};

const DEFAULT_WECHAT_BASE_URL: &str = "https://ilinkai.weixin.qq.com";
const WECHAT_CDN_BASE_URL: &str = "https://novac2c.cdn.weixin.qq.com/c2c";
const DEFAULT_POLLING_TIMEOUT_SECS: u64 = 40;

type Aes128EcbDec = ecb::Decryptor<Aes128>;

#[derive(Debug)]
pub struct WechatChannel {
    client: Client,
    token: String,
    user_id: String,
    get_updates_buf: Mutex<Option<String>>,
}

impl WechatChannel {
    pub fn new(config: WechatChannelConfig) -> BabataResult<Self> {
        let get_updates_buf = Self::load_get_updates_buf()?;

        Ok(Self {
            client: Client::new(),
            token: config.token,
            user_id: config.user_id,
            get_updates_buf: Mutex::new(get_updates_buf),
        })
    }

    async fn fetch_updates(&self) -> BabataResult<Vec<WechatIncomingMessage>> {
        let mut body = serde_json::json!({
            "base_info": {
                "channel_version": env!("CARGO_PKG_VERSION"),
            }
        });
        if let Some(buf) = self.get_updates_buf.lock().await.clone() {
            body["get_updates_buf"] = Value::String(buf);
        }

        let response = self
            .send_request(
                "ilink/bot/getupdates",
                &body,
                Duration::from_secs(DEFAULT_POLLING_TIMEOUT_SECS),
            )
            .await?;
        let payload: WechatGetUpdatesPayload = serde_json::from_value(response).map_err(|err| {
            BabataError::channel(format!("Failed to parse Wechat getupdates response: {err}"))
        })?;

        if let Some(new_buf) = payload.get_updates_buf {
            self.update_get_updates_buf(new_buf).await?;
        }

        Ok(payload
            .msg_list
            .into_iter()
            .chain(payload.msgs)
            .map(WechatIncomingMessage::from)
            .collect())
    }

    async fn route_incoming(&self, incoming: Vec<WechatIncomingMessage>) -> Vec<Content> {
        let mut content = Vec::new();

        for message in incoming {
            if message.conversation.user_id != self.user_id {
                continue;
            }

            if let Err(err) = self.persist_latest_context_token(&message.conversation.context_token)
            {
                warn!(
                    "Failed to persist latest Wechat context_token for user '{}': {}",
                    message.conversation.user_id, err
                );
            }

            let Some(message_content) = self.incoming_message_to_content(&message).await else {
                warn!(
                    "Wechat message from '{}' produced no content after parsing; context_token='{}', item_count={}",
                    message.conversation.user_id,
                    message.conversation.context_token,
                    message.items.len()
                );
                continue;
            };
            content.extend(message_content);
        }

        content
    }

    async fn update_get_updates_buf(&self, get_updates_buf: String) -> BabataResult<()> {
        {
            let mut current = self.get_updates_buf.lock().await;
            if current.as_deref() == Some(get_updates_buf.as_str()) {
                return Ok(());
            }
            *current = Some(get_updates_buf.clone());
        }

        self.persist_get_updates_buf(&get_updates_buf)
    }

    fn persist_get_updates_buf(&self, get_updates_buf: &str) -> BabataResult<()> {
        let path = Self::get_updates_buf_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                BabataError::internal(format!(
                    "Failed to create Wechat channel state directory '{}': {}",
                    parent.display(),
                    err
                ))
            })?;
        }

        std::fs::write(&path, get_updates_buf).map_err(|err| {
            BabataError::internal(format!(
                "Failed to persist Wechat get_updates_buf to '{}': {}",
                path.display(),
                err
            ))
        })?;

        Ok(())
    }

    fn persist_latest_context_token(&self, context_token: &str) -> BabataResult<()> {
        let path = wechat_latest_context_token_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                BabataError::internal(format!(
                    "Failed to create Wechat channel state directory '{}': {}",
                    parent.display(),
                    err
                ))
            })?;
        }

        std::fs::write(&path, context_token).map_err(|err| {
            BabataError::internal(format!(
                "Failed to persist Wechat latest context_token to '{}': {}",
                path.display(),
                err
            ))
        })?;

        Ok(())
    }

    fn load_get_updates_buf() -> BabataResult<Option<String>> {
        let path = Self::get_updates_buf_path()?;
        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&path).map_err(|err| {
            BabataError::internal(format!(
                "Failed to read Wechat get_updates_buf from '{}': {}",
                path.display(),
                err
            ))
        })?;
        let content = content.trim();
        if content.is_empty() {
            return Ok(None);
        }

        Ok(Some(content.to_string()))
    }

    fn get_updates_buf_path() -> BabataResult<PathBuf> {
        Ok(babata_dir()?
            .join("channels")
            .join("wechat")
            .join("get_updates_buf"))
    }

    async fn incoming_message_to_content(
        &self,
        message: &WechatIncomingMessage,
    ) -> Option<Vec<Content>> {
        let mut content = Vec::new();

        let text = body_from_items(&message.items);
        if !text.is_empty() {
            content.push(Content::Text { text });
        }

        for attachment in top_level_attachments(&message.items) {
            match self.download_attachment(attachment).await {
                Ok(downloaded) => {
                    if attachment.kind == WechatAttachmentKind::Image {
                        let media_type = MediaType::from_mime(&downloaded.mime_type)
                            .or_else(|| detect_image_media_type(&downloaded.data));
                        if let Some(media_type) = media_type {
                            content.push(Content::ImageData {
                                data: base64::engine::general_purpose::STANDARD
                                    .encode(&downloaded.data),
                                media_type,
                            });
                        } else {
                            warn!(
                                "Unsupported Wechat image media type '{}'; storing attachment metadata instead.",
                                downloaded.mime_type
                            );
                            match self.persist_attachment(attachment, &downloaded.data) {
                                Ok(path) => content.push(Content::Text {
                                    text: attachment_info_text(
                                        attachment,
                                        Some(&path),
                                        Some(&downloaded.mime_type),
                                        None,
                                    ),
                                }),
                                Err(err) => {
                                    warn!(
                                        "Failed to persist unsupported Wechat image attachment '{}': {}",
                                        attachment.file_name.as_deref().unwrap_or("unknown"),
                                        err
                                    );
                                    content.push(Content::Text {
                                        text: attachment_info_text(
                                            attachment,
                                            None,
                                            Some(&downloaded.mime_type),
                                            Some(&err.to_string()),
                                        ),
                                    });
                                }
                            }
                        }
                    } else {
                        match self.persist_attachment(attachment, &downloaded.data) {
                            Ok(path) => content.push(Content::Text {
                                text: attachment_info_text(
                                    attachment,
                                    Some(&path),
                                    Some(&downloaded.mime_type),
                                    None,
                                ),
                            }),
                            Err(err) => {
                                warn!(
                                    "Failed to persist Wechat attachment '{}' ({:?}): {}",
                                    attachment.file_name.as_deref().unwrap_or("unknown"),
                                    attachment.kind,
                                    err
                                );
                                content.push(Content::Text {
                                    text: attachment_info_text(
                                        attachment,
                                        None,
                                        Some(&downloaded.mime_type),
                                        Some(&err.to_string()),
                                    ),
                                });
                            }
                        }
                    }
                }
                Err(err) => {
                    warn!(
                        "Failed to download Wechat attachment '{}' ({:?}): {}",
                        attachment.file_name.as_deref().unwrap_or("unknown"),
                        attachment.kind,
                        err
                    );
                    content.push(Content::Text {
                        text: attachment_info_text(
                            attachment,
                            None,
                            attachment.mime_type.as_deref(),
                            Some(&err.to_string()),
                        ),
                    });
                }
            }
        }

        (!content.is_empty()).then_some(content)
    }

    async fn send_request(
        &self,
        path: &str,
        body: &Value,
        timeout: Duration,
    ) -> BabataResult<Value> {
        let url = format!("{}/{}", DEFAULT_WECHAT_BASE_URL, path);
        let response = self
            .client
            .post(&url)
            .headers(self.headers()?)
            .json(body)
            .timeout(timeout)
            .send()
            .await
            .map_err(|err| {
                BabataError::channel(format!(
                    "Failed to send Wechat request to '{}': {}",
                    url, err
                ))
            })?;

        let status = response.status();
        if status != StatusCode::OK {
            let response_text = response.text().await.unwrap_or_default();
            return Err(BabataError::channel(format!(
                "Wechat API '{}' returned status {}: {}",
                path, status, response_text
            )));
        }

        let response = response.json::<Value>().await.map_err(|err| {
            BabataError::channel(format!(
                "Failed to deserialize Wechat response for '{}': {}",
                path, err
            ))
        })?;
        check_api_error(&response)?;
        Ok(response)
    }

    fn headers(&self) -> BabataResult<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("authorizationtype"),
            HeaderValue::from_static("ilink_bot_token"),
        );
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.token)).map_err(|err| {
                BabataError::channel(format!("Invalid Wechat authorization header: {err}"))
            })?,
        );
        headers.insert(
            HeaderName::from_static("x-wechat-uin"),
            HeaderValue::from_str(&random_wechat_uin()).map_err(|err| {
                BabataError::channel(format!("Invalid Wechat x-wechat-uin header: {err}"))
            })?,
        );

        Ok(headers)
    }

    async fn download_attachment(
        &self,
        attachment: &WechatAttachment,
    ) -> BabataResult<DownloadedWechatAttachment> {
        if let Some(encrypted_query_param) = attachment.encrypted_query_param.as_deref() {
            let encrypted = self
                .download_cdn_buffer(encrypted_query_param, &attachment.file_key)
                .await?;
            let mime_type = attachment.mime_type.clone().unwrap_or_else(|| {
                mime_guess::from_path(
                    attachment
                        .file_name
                        .as_deref()
                        .unwrap_or(attachment.kind.default_file_name()),
                )
                .first_or_octet_stream()
                .to_string()
            });

            if attachment.aes_key_candidates.is_empty() {
                if attachment.kind == WechatAttachmentKind::Image {
                    return Ok(DownloadedWechatAttachment {
                        mime_type,
                        data: encrypted,
                    });
                }

                return Err(BabataError::channel(format!(
                    "Wechat attachment '{}' is missing aes_key for encrypted CDN media",
                    attachment.file_key
                )));
            }

            let mut last_err = None;
            for candidate in &attachment.aes_key_candidates {
                let key = match parse_wechat_aes_key(candidate) {
                    Ok(key) => key,
                    Err(err) => {
                        last_err = Some(err.to_string());
                        continue;
                    }
                };

                match decrypt_wechat_media(&encrypted, key) {
                    Ok(data) => {
                        return Ok(DownloadedWechatAttachment { mime_type, data });
                    }
                    Err(err) => {
                        last_err = Some(err.to_string());
                    }
                }
            }

            return Err(BabataError::channel(format!(
                "Failed to decrypt Wechat attachment '{}' with {} aes key candidate(s): {}",
                attachment.file_key,
                attachment.aes_key_candidates.len(),
                last_err.unwrap_or_else(|| "unknown error".to_string())
            )));
        }

        if let Some(download_url) = attachment
            .download_url
            .as_deref()
            .filter(|url| url.starts_with("http://") || url.starts_with("https://"))
        {
            let response = self.client.get(download_url).send().await.map_err(|err| {
                BabataError::channel(format!(
                    "Failed to download Wechat attachment from url '{}': {}",
                    download_url, err
                ))
            })?;
            let response = response.error_for_status().map_err(|err| {
                BabataError::channel(format!(
                    "Wechat attachment url download failed for '{}': {}",
                    download_url, err
                ))
            })?;
            let data = response.bytes().await.map_err(|err| {
                BabataError::channel(format!(
                    "Failed to read Wechat attachment bytes from url '{}': {}",
                    download_url, err
                ))
            })?;
            let mime_type = attachment.mime_type.clone().unwrap_or_else(|| {
                mime_guess::from_path(
                    attachment
                        .file_name
                        .as_deref()
                        .unwrap_or(attachment.kind.default_file_name()),
                )
                .first_or_octet_stream()
                .to_string()
            });

            return Ok(DownloadedWechatAttachment {
                mime_type,
                data: data.to_vec(),
            });
        }

        Err(BabataError::channel(format!(
            "Wechat attachment '{}' is missing a supported download source",
            attachment.file_key
        )))
    }

    async fn download_cdn_buffer(
        &self,
        encrypted_query_param: &str,
        file_key: &str,
    ) -> BabataResult<Vec<u8>> {
        let url = build_wechat_cdn_download_url(encrypted_query_param)?;
        let response = self.client.get(url.clone()).send().await.map_err(|err| {
            BabataError::channel(format!(
                "Failed to download Wechat attachment '{}' from CDN '{}': {}",
                file_key, url, err
            ))
        })?;
        let response = response.error_for_status().map_err(|err| {
            BabataError::channel(format!(
                "Wechat CDN download failed for attachment '{}' via '{}': {}",
                file_key, url, err
            ))
        })?;

        response
            .bytes()
            .await
            .map(|bytes| bytes.to_vec())
            .map_err(|err| {
                BabataError::channel(format!(
                    "Failed to read Wechat CDN attachment bytes for '{}' via '{}': {}",
                    file_key, url, err
                ))
            })
    }

    fn persist_attachment(
        &self,
        attachment: &WechatAttachment,
        data: &[u8],
    ) -> BabataResult<PathBuf> {
        let dir = Self::media_dir()?;
        std::fs::create_dir_all(&dir).map_err(|err| {
            BabataError::internal(format!(
                "Failed to create Wechat media directory '{}': {}",
                dir.display(),
                err
            ))
        })?;

        let file_name = sanitize_file_name(
            attachment
                .file_name
                .as_deref()
                .unwrap_or(attachment.kind.default_file_name()),
        );
        let path = dir.join(format!("{}_{}", Uuid::new_v4(), file_name));
        std::fs::write(&path, data).map_err(|err| {
            BabataError::internal(format!(
                "Failed to persist Wechat attachment to '{}': {}",
                path.display(),
                err
            ))
        })?;

        Ok(path)
    }

    fn media_dir() -> BabataResult<PathBuf> {
        Ok(babata_dir()?.join("channels").join("wechat").join("media"))
    }
}

pub(crate) fn wechat_latest_context_token_path() -> BabataResult<PathBuf> {
    Ok(wechat_latest_context_token_path_in(&babata_dir()?))
}

pub(crate) fn wechat_latest_context_token_path_in(babata_home: &Path) -> PathBuf {
    babata_home
        .join("channels")
        .join("wechat")
        .join("latest_context_token")
}

#[async_trait::async_trait]
impl super::Channel for WechatChannel {
    fn name() -> &'static str {
        "Wechat"
    }

    async fn try_receive(&self) -> BabataResult<Vec<Content>> {
        let incoming = self.fetch_updates().await?;
        Ok(self.route_incoming(incoming).await)
    }

    async fn feedback(&self, content: Vec<Content>) -> BabataResult<Vec<Content>> {
        let _ = content;
        unimplemented!("Wechat feedback is not implemented yet")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WechatConversation {
    user_id: String,
    context_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WechatIncomingMessage {
    conversation: WechatConversation,
    items: Vec<WechatMessageItem>,
}

impl From<RawWechatIncomingMessage> for WechatIncomingMessage {
    fn from(value: RawWechatIncomingMessage) -> Self {
        Self {
            conversation: WechatConversation {
                user_id: if !value.from_user_id.trim().is_empty() {
                    value.from_user_id
                } else {
                    value.to_user_id
                },
                context_token: value.context_token,
            },
            items: parse_items(value.item_list),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WechatMessageItem {
    Text(String),
    VoiceTranscription(String),
    Quote(Vec<WechatMessageItem>),
    Attachment(WechatAttachment),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WechatAttachmentKind {
    Image,
    Video,
    File,
    Audio,
}

impl WechatAttachmentKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Video => "video",
            Self::File => "file",
            Self::Audio => "audio",
        }
    }

    fn default_file_name(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Video => "video",
            Self::File => "file",
            Self::Audio => "audio",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WechatAttachment {
    kind: WechatAttachmentKind,
    file_key: String,
    encrypted_query_param: Option<String>,
    aes_key: Option<String>,
    aes_key_candidates: Vec<String>,
    download_url: Option<String>,
    file_name: Option<String>,
    file_size: Option<u64>,
    mime_type: Option<String>,
}

#[derive(Debug)]
struct DownloadedWechatAttachment {
    mime_type: String,
    data: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct WechatGetUpdatesPayload {
    #[serde(default)]
    get_updates_buf: Option<String>,
    #[serde(default)]
    msg_list: Vec<RawWechatIncomingMessage>,
    #[serde(default)]
    msgs: Vec<RawWechatIncomingMessage>,
}

#[derive(Debug, Deserialize)]
struct RawWechatIncomingMessage {
    #[serde(default)]
    from_user_id: String,
    #[serde(default)]
    to_user_id: String,
    #[serde(default)]
    context_token: String,
    #[serde(default)]
    item_list: Vec<RawWechatItem>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawWechatItem {
    #[serde(rename = "type")]
    item_type: u8,
    #[serde(default)]
    body: Option<Value>,
    #[serde(default)]
    text_item: Option<WechatTextItem>,
    #[serde(default)]
    voice_transcription_body: Option<String>,
    #[serde(default)]
    ref_item_list: Vec<RawWechatItem>,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize)]
struct WechatTextItem {
    #[serde(default)]
    text: String,
}

impl RawWechatItem {
    fn attachment(&self, kind: WechatAttachmentKind) -> Option<WechatAttachment> {
        let aes_key_candidates = self.aes_key_candidates();
        let aes_key = aes_key_candidates.first().cloned();
        let encrypted_query_param = self.string_path_field(&[
            &["encrypt_query_param"],
            &["media", "encrypt_query_param"],
            &["image_item", "encrypt_query_param"],
            &["image_item", "media", "encrypt_query_param"],
            &["video_item", "encrypt_query_param"],
            &["video_item", "media", "encrypt_query_param"],
            &["file_item", "encrypt_query_param"],
            &["file_item", "media", "encrypt_query_param"],
            &["audio_item", "encrypt_query_param"],
            &["audio_item", "media", "encrypt_query_param"],
        ]);
        let download_url = self.string_path_field(&[
            &["url"],
            &["image_item", "url"],
            &["video_item", "url"],
            &["file_item", "url"],
            &["audio_item", "url"],
        ]);
        let file_key = self
            .string_path_field(&[
                &["filekey"],
                &["file_key"],
                &["media", "filekey"],
                &["media", "file_key"],
                &["image_item", "filekey"],
                &["image_item", "file_key"],
                &["image_item", "media", "filekey"],
                &["image_item", "media", "file_key"],
                &["video_item", "filekey"],
                &["video_item", "file_key"],
                &["video_item", "media", "filekey"],
                &["video_item", "media", "file_key"],
                &["file_item", "filekey"],
                &["file_item", "file_key"],
                &["file_item", "media", "filekey"],
                &["file_item", "media", "file_key"],
                &["audio_item", "filekey"],
                &["audio_item", "file_key"],
                &["audio_item", "media", "filekey"],
                &["audio_item", "media", "file_key"],
            ])
            .or_else(|| encrypted_query_param.clone())
            .or_else(|| download_url.clone())?;
        Some(WechatAttachment {
            kind,
            file_key,
            encrypted_query_param,
            aes_key,
            aes_key_candidates,
            download_url,
            file_name: self.string_path_field(&[
                &["file_name"],
                &["image_item", "file_name"],
                &["video_item", "file_name"],
                &["file_item", "file_name"],
                &["audio_item", "file_name"],
            ]),
            file_size: self.u64_path_field(&[
                &["file_size"],
                &["mid_size"],
                &["len"],
                &["image_item", "file_size"],
                &["image_item", "mid_size"],
                &["video_item", "file_size"],
                &["video_item", "len"],
                &["file_item", "file_size"],
                &["file_item", "len"],
                &["audio_item", "file_size"],
                &["audio_item", "len"],
            ]),
            mime_type: self.string_path_field(&[
                &["mime_type"],
                &["image_item", "mime_type"],
                &["video_item", "mime_type"],
                &["file_item", "mime_type"],
                &["audio_item", "mime_type"],
            ]),
        })
    }

    fn aes_key_candidates(&self) -> Vec<String> {
        let mut candidates = Vec::new();
        for value in self.string_path_fields(&[
            &["aeskey"],
            &["image_item", "aeskey"],
            &["video_item", "aeskey"],
            &["file_item", "aeskey"],
            &["audio_item", "aeskey"],
            &["aes_key"],
            &["media", "aes_key"],
            &["image_item", "aes_key"],
            &["image_item", "media", "aes_key"],
            &["video_item", "aes_key"],
            &["video_item", "media", "aes_key"],
            &["file_item", "aes_key"],
            &["file_item", "media", "aes_key"],
            &["audio_item", "aes_key"],
            &["audio_item", "media", "aes_key"],
        ]) {
            push_unique(&mut candidates, value.clone());
            if let Some(decoded) = decode_base64_hex_key(&value) {
                push_unique(&mut candidates, decoded);
            }
        }
        candidates
    }

    fn string_path_field(&self, paths: &[&[&str]]) -> Option<String> {
        paths.iter().find_map(|path| {
            self.path_field(path)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
        })
    }

    fn string_path_fields(&self, paths: &[&[&str]]) -> Vec<String> {
        let mut values = Vec::new();
        for path in paths {
            if let Some(value) = self
                .path_field(path)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                push_unique(&mut values, value.to_string());
            }
        }
        values
    }

    fn u64_path_field(&self, paths: &[&[&str]]) -> Option<u64> {
        paths.iter().find_map(|path| {
            let value = self.path_field(path)?;
            value
                .as_u64()
                .or_else(|| value.as_str().and_then(|value| value.parse::<u64>().ok()))
        })
    }

    fn path_field(&self, path: &[&str]) -> Option<&Value> {
        if path.is_empty() {
            return None;
        }

        self.path_from_value(self.body.as_ref(), path)
            .or_else(|| self.path_from_map(&self.extra, path))
    }

    fn path_from_value<'a>(&'a self, value: Option<&'a Value>, path: &[&str]) -> Option<&'a Value> {
        let mut current = value?;
        for segment in path {
            current = current.as_object()?.get(*segment)?;
        }
        Some(current)
    }

    fn path_from_map<'a>(
        &'a self,
        map: &'a HashMap<String, Value>,
        path: &[&str],
    ) -> Option<&'a Value> {
        let (first, rest) = path.split_first()?;
        let mut current = map.get(*first)?;
        for segment in rest {
            current = current.as_object()?.get(*segment)?;
        }
        Some(current)
    }

    fn debug_field_paths(&self) -> Vec<String> {
        let mut paths = Vec::new();
        if let Some(body) = &self.body {
            collect_value_paths("body", body, &mut paths);
        }
        for (key, value) in &self.extra {
            collect_value_paths(key, value, &mut paths);
        }
        paths.sort();
        paths.dedup();
        paths
    }
}

fn parse_items(raw_items: Vec<RawWechatItem>) -> Vec<WechatMessageItem> {
    let mut items = Vec::new();

    for raw in raw_items {
        match raw.item_type {
            0 => {
                if let Some(text) = extract_text_item(&raw) {
                    items.push(WechatMessageItem::Text(text));
                }
            }
            1 => {
                if let Some(attachment) = raw.attachment(WechatAttachmentKind::Image) {
                    items.push(WechatMessageItem::Attachment(attachment));
                } else if let Some(text) = extract_text_item(&raw) {
                    items.push(WechatMessageItem::Text(text));
                } else {
                    warn!(
                        "Unhandled Wechat item type=1; field_paths={}",
                        raw.debug_field_paths().join(",")
                    );
                }
            }
            2 => {
                if let Some(attachment) = raw.attachment(WechatAttachmentKind::Image) {
                    items.push(WechatMessageItem::Attachment(attachment));
                } else if let Some(attachment) = raw.attachment(WechatAttachmentKind::Video) {
                    items.push(WechatMessageItem::Attachment(attachment));
                } else {
                    warn!(
                        "Unhandled Wechat item type=2; field_paths={}",
                        raw.debug_field_paths().join(",")
                    );
                }
            }
            3 => {
                if let Some(attachment) = raw.attachment(WechatAttachmentKind::File) {
                    items.push(WechatMessageItem::Attachment(attachment));
                } else {
                    warn!(
                        "Unhandled Wechat item type=3; field_paths={}",
                        raw.debug_field_paths().join(",")
                    );
                }
            }
            4 | 5 | 6 => {
                let mut handled = false;
                if let Some(attachment) = raw.attachment(WechatAttachmentKind::Audio) {
                    items.push(WechatMessageItem::Attachment(attachment));
                    handled = true;
                }
                if let Some(ref transcription) = raw.voice_transcription_body {
                    let transcription = transcription.trim();
                    if !transcription.is_empty() {
                        items.push(WechatMessageItem::VoiceTranscription(
                            transcription.to_string(),
                        ));
                        handled = true;
                    }
                }
                if !handled {
                    warn!(
                        "Unhandled Wechat item type={}; field_paths={}",
                        raw.item_type,
                        raw.debug_field_paths().join(",")
                    );
                }
            }
            7 => {
                let quoted = parse_items(raw.ref_item_list);
                if !quoted.is_empty() {
                    items.push(WechatMessageItem::Quote(quoted));
                }
            }
            _ => {
                warn!(
                    "Unhandled Wechat item type={}; field_paths={}",
                    raw.item_type,
                    raw.debug_field_paths().join(",")
                );
            }
        }
    }

    items
}

fn extract_text_item(item: &RawWechatItem) -> Option<String> {
    if let Some(text) = item
        .text_item
        .as_ref()
        .map(|text_item| text_item.text.trim())
        .filter(|text| !text.is_empty())
    {
        return Some(text.to_string());
    }

    item.body
        .as_ref()
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
}

fn body_from_items(items: &[WechatMessageItem]) -> String {
    let mut parts = Vec::new();

    for item in items {
        match item {
            WechatMessageItem::Text(text) | WechatMessageItem::VoiceTranscription(text) => {
                let text = text.trim();
                if !text.is_empty() {
                    parts.push(text.to_string());
                }
            }
            WechatMessageItem::Quote(quoted) => {
                let quoted_text = body_from_items(quoted);
                if !quoted_text.is_empty() {
                    parts.push(format!("> {quoted_text}"));
                }
            }
            WechatMessageItem::Attachment(_) => {}
        }
    }

    parts.join("\n")
}

fn top_level_attachments(items: &[WechatMessageItem]) -> Vec<&WechatAttachment> {
    items
        .iter()
        .filter_map(|item| match item {
            WechatMessageItem::Attachment(attachment) => Some(attachment),
            WechatMessageItem::Text(_)
            | WechatMessageItem::VoiceTranscription(_)
            | WechatMessageItem::Quote(_) => None,
        })
        .collect()
}

fn attachment_info_text(
    attachment: &WechatAttachment,
    local_path: Option<&Path>,
    mime_type: Option<&str>,
    download_error: Option<&str>,
) -> String {
    let mut object = serde_json::Map::new();
    object.insert(
        "type".to_string(),
        Value::String("wechat_attachment".to_string()),
    );
    object.insert(
        "kind".to_string(),
        Value::String(attachment.kind.as_str().to_string()),
    );
    object.insert(
        "file_key".to_string(),
        Value::String(attachment.file_key.clone()),
    );

    if let Some(file_name) = &attachment.file_name {
        object.insert("file_name".to_string(), Value::String(file_name.clone()));
    }
    if let Some(file_size) = attachment.file_size {
        object.insert("file_size".to_string(), Value::Number(file_size.into()));
    }
    if let Some(mime_type) = mime_type {
        object.insert(
            "mime_type".to_string(),
            Value::String(mime_type.to_string()),
        );
    }
    if let Some(local_path) = local_path {
        object.insert(
            "local_path".to_string(),
            Value::String(local_path.display().to_string()),
        );
    }
    if let Some(download_error) = download_error {
        object.insert(
            "download_error".to_string(),
            Value::String(download_error.to_string()),
        );
    }

    Value::Object(object).to_string()
}

fn check_api_error(response: &Value) -> BabataResult<()> {
    if let Some(code) = response.get("errcode").and_then(Value::as_i64)
        && code != 0
    {
        let message = response
            .get("errmsg")
            .and_then(Value::as_str)
            .unwrap_or("unknown error");
        return Err(BabataError::channel(format!(
            "Wechat API error {}: {}",
            code, message
        )));
    }

    Ok(())
}

fn random_wechat_uin() -> String {
    STANDARD.encode(rand::random::<u32>().to_string())
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

fn build_wechat_cdn_download_url(encrypted_query_param: &str) -> BabataResult<Url> {
    Url::parse_with_params(
        &format!("{WECHAT_CDN_BASE_URL}/download"),
        [("encrypted_query_param", encrypted_query_param)],
    )
    .map_err(|err| {
        BabataError::channel(format!(
            "Failed to build Wechat CDN download url for encrypted_query_param: {}",
            err
        ))
    })
}

fn decrypt_wechat_media(ciphertext: &[u8], key: [u8; 16]) -> BabataResult<Vec<u8>> {
    let dec = Aes128EcbDec::new((&key).into());
    let mut buffer = ciphertext.to_vec();
    let plaintext = dec
        .decrypt_padded_mut::<Pkcs7>(&mut buffer)
        .map_err(|err| {
            BabataError::channel(format!(
                "Failed to decrypt Wechat media with AES-128-ECB: {err}"
            ))
        })?;
    Ok(plaintext.to_vec())
}

fn parse_wechat_aes_key(value: &str) -> BabataResult<[u8; 16]> {
    let value = value.trim();
    if let Some(key) = decode_hex_aes_key(value) {
        return Ok(key);
    }

    let decoded = STANDARD.decode(value).map_err(|err| {
        BabataError::channel(format!(
            "Invalid Wechat aes_key base64 '{}': {}",
            value, err
        ))
    })?;
    if decoded.len() == 16 {
        return decoded.try_into().map_err(|_| {
            BabataError::channel("Wechat aes_key must decode to 16 bytes".to_string())
        });
    }

    let decoded_text = std::str::from_utf8(&decoded).map_err(|err| {
        BabataError::channel(format!(
            "Wechat aes_key base64 is neither raw bytes nor utf-8 hex: {}",
            err
        ))
    })?;
    decode_hex_aes_key(decoded_text).ok_or_else(|| {
        BabataError::channel(
            "Wechat aes_key must decode to 16 raw bytes or a 32-char hex string".to_string(),
        )
    })
}

fn decode_hex_aes_key(value: &str) -> Option<[u8; 16]> {
    let value = value.trim();
    if value.len() != 32 || !value.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }

    let decoded = hex::decode(value).ok()?;
    decoded.try_into().ok()
}

fn decode_base64_hex_key(value: &str) -> Option<String> {
    let decoded = STANDARD.decode(value).ok()?;
    let decoded = String::from_utf8(decoded).ok()?;
    let decoded = decoded.trim();
    if decoded.len() == 32 && decoded.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Some(decoded.to_ascii_lowercase())
    } else {
        None
    }
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn collect_value_paths(prefix: &str, value: &Value, out: &mut Vec<String>) {
    out.push(prefix.to_string());
    if let Some(object) = value.as_object() {
        for (key, nested) in object {
            collect_value_paths(&format!("{prefix}.{key}"), nested, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_items_extracts_typed_text_quote_and_attachments() {
        let raw: RawWechatIncomingMessage = serde_json::from_value(serde_json::json!({
            "from_user_id": "wxid_user",
            "to_user_id": "wxid_user",
            "context_token": "ctx_1",
            "item_list": [
                { "type": 0, "body": "hello" },
                {
                    "type": 1,
                    "body": {
                        "filekey": "image-key",
                        "aes_key": "00112233445566778899aabbccddeeff",
                        "file_name": "cat.png",
                        "file_size": 42,
                        "mime_type": "image/png"
                    }
                },
                {
                    "type": 3,
                    "body": {
                        "filekey": "file-key",
                        "aes_key": "00112233445566778899aabbccddeeff",
                        "file_name": "report.pdf",
                        "file_size": 512
                    }
                },
                {
                    "type": 7,
                    "ref_item_list": [
                        { "type": 0, "body": "quoted" }
                    ]
                }
            ]
        }))
        .expect("parse raw message");

        let message = WechatIncomingMessage::from(raw);

        assert_eq!(message.conversation.user_id, "wxid_user");
        assert_eq!(message.conversation.context_token, "ctx_1");
        assert_eq!(message.items.len(), 4);
        assert!(matches!(&message.items[0], WechatMessageItem::Text(text) if text == "hello"));
        assert!(matches!(
            &message.items[1],
            WechatMessageItem::Attachment(WechatAttachment {
                kind: WechatAttachmentKind::Image,
                ..
            })
        ));
        assert!(matches!(
            &message.items[2],
            WechatMessageItem::Attachment(WechatAttachment {
                kind: WechatAttachmentKind::File,
                ..
            })
        ));
        assert!(matches!(&message.items[3], WechatMessageItem::Quote(items) if items.len() == 1));
    }

    #[test]
    fn parse_items_extracts_nested_image_item_media() {
        let raw: RawWechatIncomingMessage = serde_json::from_value(serde_json::json!({
            "from_user_id": "wxid_user",
            "to_user_id": "wxid_user",
            "context_token": "ctx_nested",
            "item_list": [
                {
                    "type": 1,
                    "body": {
                        "image_item": {
                            "file_name": "nested.png",
                            "mid_size": 64,
                            "media": {
                                "filekey": "nested-image-key",
                                "aes_key": "00112233445566778899aabbccddeeff"
                            }
                        }
                    }
                }
            ]
        }))
        .expect("parse raw nested image message");

        let message = WechatIncomingMessage::from(raw);

        assert_eq!(message.items.len(), 1);
        assert!(matches!(
            &message.items[0],
            WechatMessageItem::Attachment(WechatAttachment {
                kind: WechatAttachmentKind::Image,
                file_key,
                file_name,
                file_size,
                ..
            }) if file_key == "nested-image-key"
                && file_name.as_deref() == Some("nested.png")
                && *file_size == Some(64)
        ));
    }

    #[test]
    fn parse_items_extracts_type_two_image_item_shape() {
        let raw: RawWechatIncomingMessage = serde_json::from_value(serde_json::json!({
            "from_user_id": "wxid_user",
            "to_user_id": "wxid_user",
            "context_token": "ctx_type_2",
            "item_list": [
                {
                    "type": 2,
                    "image_item": {
                        "url": "https://example.com/image.png",
                        "mid_size": 128,
                        "aeskey": "00112233445566778899aabbccddeeff",
                        "media": {
                            "aes_key": "00112233445566778899aabbccddeeff",
                            "encrypt_query_param": "encrypted-param"
                        }
                    }
                }
            ]
        }))
        .expect("parse raw type 2 image message");

        let message = WechatIncomingMessage::from(raw);

        assert!(matches!(
            &message.items[0],
            WechatMessageItem::Attachment(WechatAttachment {
                kind: WechatAttachmentKind::Image,
                download_url,
                encrypted_query_param,
                file_size,
                aes_key,
                aes_key_candidates,
                ..
            }) if download_url.as_deref() == Some("https://example.com/image.png")
                && encrypted_query_param.as_deref() == Some("encrypted-param")
                && *file_size == Some(128)
                && aes_key.as_deref() == Some("00112233445566778899aabbccddeeff")
                && aes_key_candidates.iter().any(|candidate| candidate == "00112233445566778899aabbccddeeff")
        ));
    }

    #[test]
    fn body_from_items_renders_text_voice_and_quote() {
        let items = vec![
            WechatMessageItem::Text("first".to_string()),
            WechatMessageItem::VoiceTranscription("voice text".to_string()),
            WechatMessageItem::Quote(vec![WechatMessageItem::Text("quoted".to_string())]),
        ];

        let body = body_from_items(&items);

        assert_eq!(body, "first\nvoice text\n> quoted");
    }

    #[test]
    fn attachment_info_text_serializes_metadata_as_json() {
        let attachment = WechatAttachment {
            kind: WechatAttachmentKind::File,
            file_key: "file-key".to_string(),
            encrypted_query_param: Some("encrypted-param".to_string()),
            aes_key: Some("00112233445566778899aabbccddeeff".to_string()),
            aes_key_candidates: vec!["00112233445566778899aabbccddeeff".to_string()],
            download_url: None,
            file_name: Some("report.pdf".to_string()),
            file_size: Some(128),
            mime_type: Some("application/pdf".to_string()),
        };

        let text = attachment_info_text(
            &attachment,
            Some(Path::new("C:/tmp/report.pdf")),
            Some("application/pdf"),
            None,
        );
        let value: Value = serde_json::from_str(&text).expect("parse metadata text");

        assert_eq!(value["type"], "wechat_attachment");
        assert_eq!(value["kind"], "file");
        assert_eq!(value["file_name"], "report.pdf");
        assert_eq!(value["local_path"], "C:/tmp/report.pdf");
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
    fn decode_base64_hex_key_supports_media_ref_format() {
        let encoded = STANDARD.encode(b"00112233445566778899aabbccddeeff");
        assert_eq!(
            decode_base64_hex_key(&encoded).as_deref(),
            Some("00112233445566778899aabbccddeeff")
        );
    }

    #[test]
    fn parse_wechat_aes_key_supports_raw_hex_and_base64_variants() {
        let raw_key = [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ];
        let raw_b64 = "ABEiM0RVZneImaq7zN3u/w==";
        let hex_b64 = "MDAxMTIyMzM0NDU1NjY3Nzg4OTlhYWJiY2NkZGVlZmY=";

        assert_eq!(
            parse_wechat_aes_key("00112233445566778899aabbccddeeff").unwrap(),
            raw_key
        );
        assert_eq!(parse_wechat_aes_key(raw_b64).unwrap(), raw_key);
        assert_eq!(parse_wechat_aes_key(hex_b64).unwrap(), raw_key);
    }

    #[test]
    fn decrypt_wechat_media_matches_weixin_agent_sdk_behavior() {
        use cipher::BlockEncryptMut as _;

        let key = [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ];
        let plaintext = b"wechat image payload";
        let enc = ecb::Encryptor::<Aes128>::new((&key).into());
        let mut buffer = vec![0u8; plaintext.len() + 16];
        buffer[..plaintext.len()].copy_from_slice(plaintext);
        let ciphertext = enc
            .encrypt_padded_mut::<Pkcs7>(&mut buffer, plaintext.len())
            .unwrap()
            .to_vec();

        assert_eq!(decrypt_wechat_media(&ciphertext, key).unwrap(), plaintext);
    }

    #[test]
    fn incoming_message_prefers_from_user_id_for_sender() {
        let raw: RawWechatIncomingMessage = serde_json::from_value(serde_json::json!({
            "from_user_id": "wxid_sender",
            "to_user_id": "wxid_bot",
            "context_token": "ctx_2",
            "item_list": []
        }))
        .expect("parse raw message");

        let message = WechatIncomingMessage::from(raw);

        assert_eq!(message.conversation.user_id, "wxid_sender");
        assert_eq!(message.conversation.context_token, "ctx_2");
    }

    #[test]
    fn get_updates_payload_accepts_msgs_fallback() {
        let payload: WechatGetUpdatesPayload = serde_json::from_value(serde_json::json!({
            "get_updates_buf": "buf_1",
            "msgs": [
                {
                    "from_user_id": "wxid_sender",
                    "to_user_id": "wxid_bot",
                    "context_token": "ctx_3",
                    "item_list": [
                        { "type": 1, "text_item": { "text": "hello" } }
                    ]
                }
            ]
        }))
        .expect("parse payload");

        assert_eq!(payload.get_updates_buf.as_deref(), Some("buf_1"));
        assert_eq!(payload.msg_list.len(), 0);
        assert_eq!(payload.msgs.len(), 1);
        assert_eq!(payload.msgs[0].from_user_id, "wxid_sender");
    }
}
