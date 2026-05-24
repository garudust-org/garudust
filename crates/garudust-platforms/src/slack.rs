use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use dashmap::DashMap;
use futures::{SinkExt, Stream, StreamExt};
use garudust_core::{
    error::PlatformError,
    platform::{MessageHandler, PlatformAdapter},
    types::{ChannelId, DocAttachment, InboundMessage, OutboundMessage},
};
use serde::Deserialize;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub struct SlackAdapter {
    bot_token: String,
    app_token: String,
    task: Arc<std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl SlackAdapter {
    pub fn new(bot_token: String, app_token: String) -> Self {
        Self {
            bot_token,
            app_token,
            task: Arc::new(std::sync::Mutex::new(None)),
        }
    }
}

#[derive(Deserialize)]
struct SlackEnvelope {
    envelope_id: Option<String>,
    #[serde(rename = "type")]
    kind: String,
    payload: Option<EventPayload>,
}

#[derive(Deserialize)]
struct EventPayload {
    event: Option<SlackEvent>,
}

#[derive(Deserialize)]
struct SlackFile {
    id: String,
    name: Option<String>,
    filetype: Option<String>,
    url_private_download: Option<String>,
}

#[derive(Deserialize)]
struct SlackEvent {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
    user: Option<String>,
    channel: Option<String>,
    subtype: Option<String>,
    bot_id: Option<String>,
    files: Option<Vec<SlackFile>>,
}

async fn open_connection(app_token: &str) -> Result<String, PlatformError> {
    let resp: serde_json::Value = reqwest::Client::new()
        .post("https://slack.com/api/apps.connections.open")
        .header("Authorization", format!("Bearer {app_token}"))
        .header("Content-Length", "0")
        .send()
        .await
        .map_err(|e| PlatformError::Connection(e.to_string()))?
        .json()
        .await
        .map_err(|e| PlatformError::Connection(e.to_string()))?;

    if resp["ok"].as_bool() != Some(true) {
        return Err(PlatformError::Auth);
    }
    resp["url"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| PlatformError::Connection("no wss url in response".into()))
}

async fn post_message(bot_token: &str, channel: &str, text: &str) -> Result<(), PlatformError> {
    let resp: serde_json::Value = reqwest::Client::new()
        .post("https://slack.com/api/chat.postMessage")
        .header("Authorization", format!("Bearer {bot_token}"))
        .json(&serde_json::json!({ "channel": channel, "text": text }))
        .send()
        .await
        .map_err(|e| PlatformError::Send(e.to_string()))?
        .json()
        .await
        .map_err(|e| PlatformError::Send(e.to_string()))?;

    if resp["ok"].as_bool() != Some(true) {
        return Err(PlatformError::Send(
            resp["error"].as_str().unwrap_or("unknown").to_string(),
        ));
    }
    Ok(())
}

/// Fetch the display name for a Slack user via `users.info`.
/// Requires the `users:read` OAuth scope. Returns `None` on failure so
/// callers can fall back to the raw user ID.
async fn fetch_display_name(
    client: &reqwest::Client,
    bot_token: &str,
    user_id: &str,
) -> Option<String> {
    let resp: serde_json::Value = client
        .get("https://slack.com/api/users.info")
        .header("Authorization", format!("Bearer {bot_token}"))
        .query(&[("user", user_id)])
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;

    if resp["ok"].as_bool() != Some(true) {
        tracing::warn!(
            user_id,
            error = resp["error"].as_str().unwrap_or("unknown"),
            "Slack: users.info failed — falling back to userId"
        );
        return None;
    }

    let profile = &resp["user"]["profile"];
    // Prefer the user-set display_name; fall back to real_name
    profile["display_name"]
        .as_str()
        .filter(|s| !s.is_empty())
        .or_else(|| profile["real_name"].as_str())
        .filter(|s| !s.is_empty())
        .map(String::from)
}

async fn download_slack_file(
    client: &reqwest::Client,
    bot_token: &str,
    file: &SlackFile,
) -> Option<DocAttachment> {
    let url = file.url_private_download.as_deref()?;
    let file_name = file
        .name
        .clone()
        .unwrap_or_else(|| format!("{}.bin", file.id));
    let ext = std::path::Path::new(&file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_else(|| file.filetype.as_deref().unwrap_or("bin"));

    let supported = matches!(
        ext.to_lowercase().as_str(),
        "pdf" | "txt" | "csv" | "md" | "json" | "docx" | "doc" | "xlsx" | "xls"
    );
    if !supported {
        return None;
    }

    let dest = format!("/tmp/garudust_slack_{}.{ext}", file.id);
    let bytes = client
        .get(url)
        .header("Authorization", format!("Bearer {bot_token}"))
        .send()
        .await
        .ok()?
        .bytes()
        .await
        .ok()?;

    tokio::fs::write(&dest, &bytes).await.ok()?;
    Some(DocAttachment {
        path: dest,
        file_name,
    })
}

async fn socket_loop(
    wss_url: &str,
    bot_token: Arc<String>,
    client: Arc<reqwest::Client>,
    name_cache: Arc<DashMap<String, String>>,
    handler: Arc<dyn MessageHandler>,
) {
    let Ok((ws, _)) = connect_async(wss_url).await else {
        tracing::warn!("Slack: WebSocket connect failed");
        return;
    };
    let (mut write, mut read) = ws.split();

    while let Some(Ok(msg)) = read.next().await {
        let text = match msg {
            Message::Text(t) => t.to_string(),
            Message::Close(_) => break,
            _ => continue,
        };

        let Ok(env) = serde_json::from_str::<SlackEnvelope>(&text) else {
            continue;
        };

        // Acknowledge every envelope immediately
        if let Some(eid) = &env.envelope_id {
            let ack = format!(r#"{{"envelope_id":"{eid}"}}"#);
            let _ = write.send(Message::Text(ack.into())).await;
        }

        if env.kind != "events_api" {
            continue;
        }

        let Some(event) = env.payload.and_then(|p| p.event) else {
            continue;
        };

        // Allow file_share subtype; skip all other subtypes and bot messages.
        let is_file_share = event.subtype.as_deref() == Some("file_share");
        if event.kind != "message"
            || (event.subtype.is_some() && !is_file_share)
            || event.bot_id.is_some()
        {
            continue;
        }

        let (Some(user), Some(channel)) = (event.user, event.channel) else {
            continue;
        };

        let text = event.text.unwrap_or_default();

        // Download document attachments
        let mut doc_attachments: Vec<DocAttachment> = Vec::new();
        if let Some(files) = event.files {
            for file in &files {
                match download_slack_file(&client, &bot_token, file).await {
                    Some(att) => doc_attachments.push(att),
                    None => {
                        tracing::warn!(file_id = %file.id, "Slack: file download skipped or failed");
                    }
                }
            }
        }

        if text.is_empty() && doc_attachments.is_empty() {
            continue;
        }

        // Slack channel IDs: 'C' = public/private channel, 'G' = group DM (legacy), 'D' = DM
        let is_group = channel.starts_with('C') || channel.starts_with('G');

        let display_name = if let Some(cached) = name_cache.get(&user) {
            cached.clone()
        } else {
            let name = fetch_display_name(&client, &bot_token, &user)
                .await
                .unwrap_or_else(|| user.clone());
            name_cache.insert(user.clone(), name.clone());
            name
        };

        let inbound = InboundMessage {
            channel: ChannelId {
                platform: "slack".into(),
                chat_id: channel.clone(),
                thread_id: None,
            },
            user_id: user,
            user_name: display_name,
            text,
            session_key: format!("slack:{channel}"),
            is_group,
            bot_mentioned: None,
            attachments: vec![],
            doc_attachments,
        };
        let h = handler.clone();
        tokio::spawn(async move {
            let _ = h.handle(inbound).await;
        });
    }
}

#[async_trait]
impl PlatformAdapter for SlackAdapter {
    fn name(&self) -> &'static str {
        "slack"
    }

    async fn stop(&self) {
        if let Some(h) = self.task.lock().unwrap().take() {
            h.abort();
        }
    }

    async fn start(&self, handler: Arc<dyn MessageHandler>) -> Result<(), PlatformError> {
        let app_token = self.app_token.clone();
        let bot_token = Arc::new(self.bot_token.clone());
        let client = Arc::new(reqwest::Client::new());
        let name_cache: Arc<DashMap<String, String>> = Arc::new(DashMap::new());

        let handle = tokio::spawn(async move {
            loop {
                match open_connection(&app_token).await {
                    Ok(url) => {
                        tracing::info!("Slack Socket Mode connected");
                        socket_loop(
                            &url,
                            bot_token.clone(),
                            client.clone(),
                            name_cache.clone(),
                            handler.clone(),
                        )
                        .await;
                        tracing::warn!("Slack socket disconnected, reconnecting in 3 s");
                    }
                    Err(e) => {
                        tracing::error!("Slack connection error: {e}");
                    }
                }
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        });
        *self.task.lock().unwrap() = Some(handle);

        Ok(())
    }

    async fn send_message(
        &self,
        channel: &ChannelId,
        message: OutboundMessage,
    ) -> Result<(), PlatformError> {
        post_message(&self.bot_token, &channel.chat_id, &message.text).await
    }

    async fn send_stream(
        &self,
        channel: &ChannelId,
        mut stream: Pin<Box<dyn Stream<Item = String> + Send>>,
    ) -> Result<(), PlatformError> {
        let mut buf = String::new();
        while let Some(chunk) = stream.next().await {
            buf.push_str(&chunk);
        }
        self.send_message(channel, OutboundMessage::text(buf)).await
    }
}
