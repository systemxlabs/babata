use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use log::warn;
use reqwest::{
    Client, StatusCode,
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
const DEFAULT_POLLING_TIMEOUT_SECS: u64 = 40;

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
                        if let Some(media_type) = MediaType::from_mime(&downloaded.mime_type) {
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
        let path = wechat_agent_rs::media::download_media(
            &attachment.file_key,
            &attachment.aes_key,
            attachment.file_name.as_deref(),
        )
        .await
        .map_err(|err| {
            BabataError::channel(format!(
                "Failed to download Wechat attachment '{}': {}",
                attachment.file_key, err
            ))
        })?;
        let data = fs::read(&path).map_err(|err| {
            BabataError::channel(format!(
                "Failed to read downloaded Wechat attachment '{}' from '{}': {}",
                attachment.file_key,
                path.display(),
                err
            ))
        })?;
        let mime_type = attachment.mime_type.clone().unwrap_or_else(|| {
            mime_guess::from_path(&path)
                .first_or_octet_stream()
                .to_string()
        });

        Ok(DownloadedWechatAttachment { mime_type, data })
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
    aes_key: String,
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
        Some(WechatAttachment {
            kind,
            file_key: self.string_field(&["filekey", "file_key"])?,
            aes_key: self.string_field(&["aes_key"])?,
            file_name: self.string_field(&["file_name"]),
            file_size: self.u64_field(&["file_size"]),
            mime_type: self.string_field(&["mime_type"]),
        })
    }

    fn string_field(&self, names: &[&str]) -> Option<String> {
        names.iter().find_map(|name| {
            self.field(name)
                .and_then(Value::as_str)
                .map(ToString::to_string)
                .filter(|value| !value.trim().is_empty())
        })
    }

    fn u64_field(&self, names: &[&str]) -> Option<u64> {
        names
            .iter()
            .find_map(|name| self.field(name).and_then(Value::as_u64))
    }

    fn field(&self, name: &str) -> Option<&Value> {
        self.body
            .as_ref()
            .and_then(Value::as_object)
            .and_then(|body| body.get(name))
            .or_else(|| self.extra.get(name))
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
                if let Some(text) = extract_text_item(&raw) {
                    items.push(WechatMessageItem::Text(text));
                } else if let Some(attachment) = raw.attachment(WechatAttachmentKind::Image) {
                    items.push(WechatMessageItem::Attachment(attachment));
                }
            }
            2 => {
                if let Some(attachment) = raw.attachment(WechatAttachmentKind::Image) {
                    items.push(WechatMessageItem::Attachment(attachment));
                } else if let Some(attachment) = raw.attachment(WechatAttachmentKind::Video) {
                    items.push(WechatMessageItem::Attachment(attachment));
                }
            }
            3 => {
                if let Some(attachment) = raw.attachment(WechatAttachmentKind::File) {
                    items.push(WechatMessageItem::Attachment(attachment));
                } else if let Some(attachment) = raw.attachment(WechatAttachmentKind::Video) {
                    items.push(WechatMessageItem::Attachment(attachment));
                }
            }
            4 => {
                if let Some(attachment) = raw.attachment(WechatAttachmentKind::File) {
                    items.push(WechatMessageItem::Attachment(attachment));
                }
            }
            5 | 6 => {
                if let Some(attachment) = raw.attachment(WechatAttachmentKind::Audio) {
                    items.push(WechatMessageItem::Attachment(attachment));
                }
                if let Some(transcription) = raw.voice_transcription_body {
                    let transcription = transcription.trim();
                    if !transcription.is_empty() {
                        items.push(WechatMessageItem::VoiceTranscription(
                            transcription.to_string(),
                        ));
                    }
                }
            }
            7 => {
                let quoted = parse_items(raw.ref_item_list);
                if !quoted.is_empty() {
                    items.push(WechatMessageItem::Quote(quoted));
                }
            }
            _ => {}
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
            aes_key: "00112233445566778899aabbccddeeff".to_string(),
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
