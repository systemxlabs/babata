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
use tokio::sync::{Mutex, oneshot};
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
    bot_token: String,
    user_id: String,
    get_updates_buf: Mutex<Option<String>>,
    // Waiters for replies to outbound feedback prompts, keyed by client_msg_id.
    feedback_waiters: Mutex<HashMap<String, oneshot::Sender<Vec<Content>>>>,
}

impl WechatChannel {
    pub fn new(config: WechatChannelConfig) -> BabataResult<Self> {
        let get_updates_buf = Self::load_get_updates_buf()?;

        Ok(Self {
            client: Client::new(),
            bot_token: config.bot_token,
            user_id: config.user_id,
            get_updates_buf: Mutex::new(get_updates_buf),
            feedback_waiters: Mutex::new(HashMap::new()),
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
            if message.message_type != WechatProtocolMessageType::User {
                continue;
            }

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

            // Check if this is a reply to a feedback message by quote content hash
            let quote_hash = extract_quote_hash(&message.items);
            if let Some(hash) = quote_hash {
                let waiter = self.feedback_waiters.lock().await.remove(&hash);
                if let Some(waiter) = waiter {
                    let _ = waiter.send(message_content);
                    continue;
                }
            }

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

    async fn send_text_message(&self, context_token: &str, text: &str) -> BabataResult<()> {
        let body = serde_json::json!({
            "msg": {
                "to_user_id": self.user_id,
                "client_id": format!("babata-{}", Uuid::new_v4().simple()),
                "message_type": 2,
                "message_state": 2,
                "context_token": context_token,
                "item_list": [
                    {
                        "type": 1,
                        "text_item": {
                            "text": text
                        }
                    }
                ]
            },
            "base_info": {
                "channel_version": env!("CARGO_PKG_VERSION")
            }
        });

        self.send_request("ilink/bot/sendmessage", &body, Duration::from_secs(30))
            .await?;

        Ok(())
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
            HeaderValue::from_str(&format!("Bearer {}", self.bot_token)).map_err(|err| {
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

pub(crate) fn load_wechat_latest_context_token() -> BabataResult<Option<String>> {
    let path = wechat_latest_context_token_path()?;
    if !path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&path).map_err(|err| {
        BabataError::channel(format!(
            "Failed to read Wechat context_token from '{}': {}",
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

pub(crate) fn wechat_latest_context_token_path() -> BabataResult<PathBuf> {
    Ok(wechat_latest_context_token_path_in(&babata_dir()?))
}

pub(crate) fn wechat_latest_context_token_path_in(babata_home: &Path) -> PathBuf {
    babata_home
        .join("channels")
        .join("wechat")
        .join("latest_context_token")
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
    fn name() -> &'static str {
        "Wechat"
    }

    async fn try_receive(&self) -> BabataResult<Vec<Content>> {
        let incoming = self.fetch_updates().await?;
        Ok(self.route_incoming(incoming).await)
    }

    async fn feedback(&self, content: Vec<Content>) -> BabataResult<Vec<Content>> {
        let text = render_feedback_text(&content)?;

        // Load the latest context_token
        let context_token = load_wechat_latest_context_token()?.ok_or_else(|| {
            BabataError::channel(
                "Wechat context_token not found; no messages have been received yet".to_string(),
            )
        })?;

        // Send the feedback message
        self.send_text_message(&context_token, &text).await?;

        // Wait for user's reply
        let (sender, receiver) = oneshot::channel();
        let key = hash_feedback_key(&text);
        self.feedback_waiters.lock().await.insert(key, sender);

        receiver.await.map_err(|_| {
            BabataError::channel("Wechat feedback waiter was dropped before reply arrived")
        })
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
    message_type: WechatProtocolMessageType,
    message_state: WechatProtocolMessageState,
    items: Vec<WechatMessageItem>,
}

impl From<WechatProtocolMessage> for WechatIncomingMessage {
    fn from(value: WechatProtocolMessage) -> Self {
        let conversation = WechatConversation {
            user_id: if !value.from_user_id.trim().is_empty() {
                value.from_user_id
            } else {
                value.to_user_id
            },
            context_token: value.context_token,
        };

        Self {
            conversation,
            message_type: value.message_type.into(),
            message_state: value.message_state.into(),
            items: parse_protocol_items(value.item_list),
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
enum WechatProtocolMessageType {
    User,
    Bot,
    Unknown(u8),
}

impl From<u8> for WechatProtocolMessageType {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::User,
            2 => Self::Bot,
            _ => Self::Unknown(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WechatProtocolMessageState {
    New,
    Generating,
    Finish,
    Unknown(u8),
}

impl From<u8> for WechatProtocolMessageState {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::New,
            1 => Self::Generating,
            2 => Self::Finish,
            _ => Self::Unknown(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WechatProtocolItemType {
    Text,
    Image,
    Voice,
    File,
    Video,
    Unknown(u8),
}

impl From<u8> for WechatProtocolItemType {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Text,
            2 => Self::Image,
            3 => Self::Voice,
            4 => Self::File,
            5 => Self::Video,
            _ => Self::Unknown(value),
        }
    }
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

impl WechatAttachment {
    fn from_image(item: &WechatImageItem) -> Option<Self> {
        let media = item.media.as_ref();
        let encrypted_query_param = media
            .and_then(|media| non_empty(media.encrypt_query_param.as_str()))
            .map(ToString::to_string);
        let download_url = item
            .url
            .as_deref()
            .and_then(non_empty)
            .map(ToString::to_string);
        let file_key = media
            .and_then(WechatCdnMedia::file_key)
            .or_else(|| encrypted_query_param.clone())
            .or_else(|| download_url.clone())?;
        let mut aes_key_candidates = Vec::new();
        if let Some(aeskey) = item.aeskey.as_deref().and_then(non_empty) {
            push_unique(&mut aes_key_candidates, aeskey.to_string());
        }
        if let Some(media_aes_key) = media
            .and_then(|media| media.aes_key.as_deref())
            .and_then(non_empty)
            .map(ToString::to_string)
        {
            push_unique(&mut aes_key_candidates, media_aes_key.clone());
            if let Some(decoded) = decode_base64_hex_key(&media_aes_key) {
                push_unique(&mut aes_key_candidates, decoded);
            }
        }

        Some(Self {
            kind: WechatAttachmentKind::Image,
            file_key,
            encrypted_query_param,
            aes_key: aes_key_candidates.first().cloned(),
            aes_key_candidates,
            download_url,
            file_name: Some(WechatAttachmentKind::Image.default_file_name().to_string()),
            file_size: item.mid_size.or(item.hd_size).or(item.thumb_size),
            mime_type: infer_mime_type_from_url(item.url.as_deref()),
        })
    }

    fn from_voice(item: &WechatVoiceItem) -> Option<Self> {
        let media = item.media.as_ref()?;
        let encrypted_query_param = non_empty(media.encrypt_query_param.as_str())?.to_string();
        let mut aes_key_candidates = Vec::new();
        push_aes_key_candidate(&mut aes_key_candidates, media.aes_key.as_deref());
        Some(Self {
            kind: WechatAttachmentKind::Audio,
            file_key: media
                .file_key()
                .unwrap_or_else(|| encrypted_query_param.clone()),
            encrypted_query_param: Some(encrypted_query_param),
            aes_key: aes_key_candidates.first().cloned(),
            aes_key_candidates,
            download_url: None,
            file_name: Some("voice.silk".to_string()),
            file_size: None,
            mime_type: Some("audio/silk".to_string()),
        })
    }

    fn from_file(item: &WechatFileItem) -> Option<Self> {
        let media = item.media.as_ref()?;
        let encrypted_query_param = non_empty(media.encrypt_query_param.as_str())?.to_string();
        let mut aes_key_candidates = Vec::new();
        push_aes_key_candidate(&mut aes_key_candidates, media.aes_key.as_deref());
        Some(Self {
            kind: WechatAttachmentKind::File,
            file_key: media
                .file_key()
                .unwrap_or_else(|| encrypted_query_param.clone()),
            encrypted_query_param: Some(encrypted_query_param),
            aes_key: aes_key_candidates.first().cloned(),
            aes_key_candidates,
            download_url: None,
            file_name: item
                .file_name
                .as_deref()
                .and_then(non_empty)
                .map(ToString::to_string),
            file_size: item.len.as_deref().and_then(parse_u64_str),
            mime_type: item
                .file_name
                .as_deref()
                .and_then(infer_mime_type_from_name),
        })
    }

    fn from_video(item: &WechatVideoItem) -> Option<Self> {
        let media = item.media.as_ref()?;
        let encrypted_query_param = non_empty(media.encrypt_query_param.as_str())?.to_string();
        let mut aes_key_candidates = Vec::new();
        push_aes_key_candidate(&mut aes_key_candidates, media.aes_key.as_deref());
        Some(Self {
            kind: WechatAttachmentKind::Video,
            file_key: media
                .file_key()
                .unwrap_or_else(|| encrypted_query_param.clone()),
            encrypted_query_param: Some(encrypted_query_param),
            aes_key: aes_key_candidates.first().cloned(),
            aes_key_candidates,
            download_url: None,
            file_name: Some("video.mp4".to_string()),
            file_size: item.video_size,
            mime_type: Some("video/mp4".to_string()),
        })
    }
}

#[derive(Debug)]
struct DownloadedWechatAttachment {
    mime_type: String,
    data: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct WechatGetUpdatesPayload {
    #[serde(default)]
    get_updates_buf: Option<String>,
    #[serde(default)]
    longpolling_timeout_ms: Option<u64>,
    #[serde(default)]
    msg_list: Vec<WechatProtocolMessage>,
    #[serde(default)]
    msgs: Vec<WechatProtocolMessage>,
}

#[derive(Debug, Clone, Deserialize)]
struct WechatProtocolMessage {
    #[serde(default)]
    from_user_id: String,
    #[serde(default)]
    to_user_id: String,
    #[serde(default)]
    message_type: u8,
    #[serde(default)]
    message_state: u8,
    #[serde(default)]
    context_token: String,
    #[serde(default)]
    item_list: Vec<WechatProtocolItem>,
}

#[derive(Debug, Clone, Deserialize)]
struct WechatProtocolItem {
    #[serde(rename = "type")]
    item_type: u8,
    #[serde(default)]
    text_item: Option<WechatTextItem>,
    #[serde(default)]
    image_item: Option<WechatImageItem>,
    #[serde(default)]
    voice_item: Option<WechatVoiceItem>,
    #[serde(default)]
    file_item: Option<WechatFileItem>,
    #[serde(default)]
    video_item: Option<WechatVideoItem>,
    #[serde(default)]
    ref_item_list: Option<Vec<WechatProtocolItem>>,
}

impl WechatProtocolItem {
    fn item_type(&self) -> WechatProtocolItemType {
        self.item_type.into()
    }
}

#[derive(Debug, Clone, Deserialize)]
struct WechatTextItem {
    #[serde(default)]
    text: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct WechatImageItem {
    #[serde(default)]
    media: Option<WechatCdnMedia>,
    #[serde(default)]
    aeskey: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    mid_size: Option<u64>,
    #[serde(default)]
    thumb_size: Option<u64>,
    #[serde(default)]
    thumb_height: Option<u64>,
    #[serde(default)]
    thumb_width: Option<u64>,
    #[serde(default)]
    hd_size: Option<u64>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct WechatVoiceItem {
    #[serde(default)]
    media: Option<WechatCdnMedia>,
    #[serde(default)]
    encode_type: Option<u64>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    playtime: Option<u64>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct WechatFileItem {
    #[serde(default)]
    media: Option<WechatCdnMedia>,
    #[serde(default)]
    file_name: Option<String>,
    #[serde(default)]
    md5: Option<String>,
    #[serde(default)]
    len: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct WechatVideoItem {
    #[serde(default)]
    media: Option<WechatCdnMedia>,
    #[serde(default)]
    video_size: Option<u64>,
    #[serde(default)]
    play_length: Option<u64>,
    #[serde(default)]
    thumb_media: Option<WechatCdnMedia>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct WechatCdnMedia {
    #[serde(default)]
    encrypt_query_param: String,
    #[serde(default)]
    aes_key: Option<String>,
    #[serde(default)]
    encrypt_type: Option<u8>,
    #[serde(default, alias = "filekey", alias = "file_key")]
    file_key: Option<String>,
}

impl WechatCdnMedia {
    fn file_key(&self) -> Option<String> {
        self.file_key
            .as_deref()
            .and_then(non_empty)
            .map(ToString::to_string)
    }
}

fn parse_protocol_items(raw_items: Vec<WechatProtocolItem>) -> Vec<WechatMessageItem> {
    let mut items = Vec::new();

    for raw in raw_items {
        if let Some(ref_items) = raw.ref_item_list.as_ref() {
            let quoted = parse_protocol_items(ref_items.clone());
            if !quoted.is_empty() {
                items.push(WechatMessageItem::Quote(quoted));
            }
        }

        match raw.item_type() {
            WechatProtocolItemType::Text => {
                if let Some(text) = raw
                    .text_item
                    .as_ref()
                    .and_then(|text_item| non_empty(text_item.text.as_str()))
                {
                    items.push(WechatMessageItem::Text(text.to_string()));
                }
            }
            WechatProtocolItemType::Image => {
                if let Some(attachment) = raw
                    .image_item
                    .as_ref()
                    .and_then(WechatAttachment::from_image)
                {
                    items.push(WechatMessageItem::Attachment(attachment));
                } else {
                    warn!("Unhandled Wechat image item: {:?}", raw);
                }
            }
            WechatProtocolItemType::Voice => {
                let mut handled = false;
                if let Some(attachment) = raw
                    .voice_item
                    .as_ref()
                    .and_then(WechatAttachment::from_voice)
                {
                    items.push(WechatMessageItem::Attachment(attachment));
                    handled = true;
                }
                if let Some(text) = raw
                    .voice_item
                    .as_ref()
                    .and_then(|voice_item| voice_item.text.as_deref())
                    .and_then(non_empty)
                {
                    items.push(WechatMessageItem::VoiceTranscription(text.to_string()));
                    handled = true;
                }
                if !handled {
                    warn!("Unhandled Wechat voice item: {:?}", raw);
                }
            }
            WechatProtocolItemType::File => {
                if let Some(attachment) =
                    raw.file_item.as_ref().and_then(WechatAttachment::from_file)
                {
                    items.push(WechatMessageItem::Attachment(attachment));
                } else {
                    warn!("Unhandled Wechat file item: {:?}", raw);
                }
            }
            WechatProtocolItemType::Video => {
                if let Some(attachment) = raw
                    .video_item
                    .as_ref()
                    .and_then(WechatAttachment::from_video)
                {
                    items.push(WechatMessageItem::Attachment(attachment));
                } else {
                    warn!("Unhandled Wechat video item: {:?}", raw);
                }
            }
            WechatProtocolItemType::Unknown(item_type) => {
                warn!("Unhandled Wechat item type={item_type}: {:?}", raw);
            }
        }
    }

    items
}

/// Extract a hash from quoted content to match against feedback prompts.
/// Returns the hash of the quoted text content if a Quote item is found.
fn extract_quote_hash(items: &[WechatMessageItem]) -> Option<String> {
    for item in items {
        if let WechatMessageItem::Quote(quoted) = item {
            let text = body_from_items(quoted);
            if !text.is_empty() {
                return Some(hash_feedback_key(&text));
            }
        }
    }
    None
}

/// Hash feedback content to use as a key in the feedback_waiters map.
fn hash_feedback_key(content: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
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

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

fn parse_u64_str(value: &str) -> Option<u64> {
    non_empty(value)?.parse::<u64>().ok()
}

fn infer_mime_type_from_name(name: &str) -> Option<String> {
    let name = non_empty(name)?;
    Some(
        mime_guess::from_path(name)
            .first_or_octet_stream()
            .to_string(),
    )
}

fn infer_mime_type_from_url(url: Option<&str>) -> Option<String> {
    let url = url.and_then(non_empty)?;
    let parsed = Url::parse(url).ok()?;
    infer_mime_type_from_name(parsed.path())
}

fn push_aes_key_candidate(values: &mut Vec<String>, value: Option<&str>) {
    let Some(value) = value.and_then(non_empty) else {
        return;
    };
    push_unique(values, value.to_string());
    if let Some(decoded) = decode_base64_hex_key(value) {
        push_unique(values, decoded);
    }
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
    for code_field in ["ret", "errcode"] {
        if let Some(code) = response.get(code_field).and_then(Value::as_i64)
            && code != 0
        {
            let message = response
                .get("errmsg")
                .or_else(|| response.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("unknown error");
            return Err(BabataError::channel(format!(
                "Wechat API error {}: {}",
                code, message
            )));
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incoming_message_parses_protocol_items() {
        let hex_b64 = STANDARD.encode(b"00112233445566778899aabbccddeeff");
        let raw: WechatProtocolMessage = serde_json::from_value(serde_json::json!({
            "from_user_id": "wxid_user",
            "to_user_id": "wxid_bot",
            "message_type": 1,
            "message_state": 2,
            "context_token": "ctx_1",
            "item_list": [
                {
                    "type": 1,
                    "text_item": { "text": "reply" },
                    "ref_item_list": [
                        {
                            "type": 1,
                            "text_item": { "text": "summary" }
                        },
                        {
                            "type": 1,
                            "text_item": { "text": "original" }
                        }
                    ]
                },
                {
                    "type": 2,
                    "image_item": {
                        "url": "https://example.com/image.png",
                        "mid_size": 128,
                        "aeskey": "00112233445566778899aabbccddeeff",
                        "media": {
                            "encrypt_query_param": "image-param",
                            "aes_key": "ABEiM0RVZneImaq7zN3u/w=="
                        }
                    }
                },
                {
                    "type": 3,
                    "voice_item": {
                        "text": "voice text",
                        "media": {
                            "encrypt_query_param": "voice-param",
                            "aes_key": hex_b64
                        }
                    }
                },
                {
                    "type": 4,
                    "file_item": {
                        "file_name": "report.pdf",
                        "len": "512",
                        "media": {
                            "encrypt_query_param": "file-param",
                            "aes_key": hex_b64
                        }
                    }
                },
                {
                    "type": 5,
                    "video_item": {
                        "video_size": 2048,
                        "media": {
                            "encrypt_query_param": "video-param",
                            "aes_key": hex_b64
                        }
                    }
                }
            ]
        }))
        .expect("parse protocol message");

        let message = WechatIncomingMessage::from(raw);

        assert_eq!(message.conversation.user_id, "wxid_user");
        assert_eq!(message.conversation.context_token, "ctx_1");
        assert_eq!(message.message_type, WechatProtocolMessageType::User);
        assert_eq!(message.message_state, WechatProtocolMessageState::Finish);
        assert_eq!(message.items.len(), 7);
        assert!(matches!(&message.items[0], WechatMessageItem::Quote(items) if items.len() == 2));
        assert!(matches!(
            &message.items[1],
            WechatMessageItem::Text(text) if text == "reply"
        ));
        assert!(matches!(
            &message.items[2],
            WechatMessageItem::Attachment(WechatAttachment {
                kind: WechatAttachmentKind::Image,
                encrypted_query_param,
                aes_key,
                file_size,
                ..
            }) if encrypted_query_param.as_deref() == Some("image-param")
                && aes_key.as_deref() == Some("00112233445566778899aabbccddeeff")
                && *file_size == Some(128)
        ));
        assert!(matches!(
            &message.items[3],
            WechatMessageItem::Attachment(WechatAttachment {
                kind: WechatAttachmentKind::Audio,
                ..
            })
        ));
        assert!(matches!(
            &message.items[4],
            WechatMessageItem::VoiceTranscription(text) if text == "voice text"
        ));
        assert!(matches!(
            &message.items[5],
            WechatMessageItem::Attachment(WechatAttachment {
                kind: WechatAttachmentKind::File,
                file_name,
                file_size,
                ..
            }) if file_name.as_deref() == Some("report.pdf") && *file_size == Some(512)
        ));
        assert!(matches!(
            &message.items[6],
            WechatMessageItem::Attachment(WechatAttachment {
                kind: WechatAttachmentKind::Video,
                file_size,
                ..
            }) if *file_size == Some(2048)
        ));
    }

    #[test]
    fn image_attachment_supports_protocol_key_shapes() {
        let item: WechatProtocolItem = serde_json::from_value(serde_json::json!({
            "type": 2,
            "image_item": {
                "url": "https://example.com/path/image.png",
                "mid_size": 128,
                "aeskey": "00112233445566778899aabbccddeeff",
                "media": {
                    "encrypt_query_param": "encrypted-param",
                    "aes_key": "ABEiM0RVZneImaq7zN3u/w=="
                }
            }
        }))
        .expect("parse image item");

        let items = parse_protocol_items(vec![item]);

        assert!(matches!(
            &items[0],
            WechatMessageItem::Attachment(WechatAttachment {
                kind: WechatAttachmentKind::Image,
                download_url,
                encrypted_query_param,
                file_size,
                mime_type,
                aes_key,
                aes_key_candidates,
                ..
            }) if download_url.as_deref() == Some("https://example.com/path/image.png")
                && encrypted_query_param.as_deref() == Some("encrypted-param")
                && *file_size == Some(128)
                && mime_type.as_deref() == Some("image/png")
                && aes_key.as_deref() == Some("00112233445566778899aabbccddeeff")
                && aes_key_candidates.iter().any(|candidate| candidate == "ABEiM0RVZneImaq7zN3u/w==")
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
    fn infer_mime_type_from_url_reads_path_suffix() {
        assert_eq!(
            infer_mime_type_from_url(Some("https://example.com/assets/demo.webp")).as_deref(),
            Some("image/webp")
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
        let raw: WechatProtocolMessage = serde_json::from_value(serde_json::json!({
            "from_user_id": "wxid_sender",
            "to_user_id": "wxid_bot",
            "context_token": "ctx_2",
            "item_list": []
        }))
        .expect("parse protocol message");

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

    #[test]
    fn check_api_error_handles_ret_field() {
        let err = check_api_error(&serde_json::json!({
            "ret": -14,
            "errmsg": "session expired"
        }))
        .expect_err("ret should fail");

        assert!(err.to_string().contains("-14"));
        assert!(err.to_string().contains("session expired"));
    }
}
