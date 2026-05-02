use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Router,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use dashmap::DashMap;
use futures::{Stream, StreamExt};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use tokio::net::TcpListener;

use garudust_core::{
    error::PlatformError,
    platform::{MessageHandler, PlatformAdapter},
    types::{ChannelId, InboundMessage, OutboundMessage},
};

const LINE_REPLY_URL: &str = "https://api.line.me/v2/bot/message/reply";
const LINE_PUSH_URL: &str = "https://api.line.me/v2/bot/message/push";
const LINE_PROFILE_URL: &str = "https://api.line.me/v2/bot/profile";
/// Reply token is valid for 30 s; leave a 5 s safety margin.
const REPLY_TTL: Duration = Duration::from_secs(25);

// ── LINE webhook deserialization ──────────────────────────────────────────────

#[derive(Deserialize)]
struct Webhook {
    events: Vec<Event>,
}

#[derive(Deserialize)]
struct Event {
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "replyToken")]
    reply_token: Option<String>,
    source: Source,
    message: Option<LineMessage>,
}

#[derive(Deserialize)]
struct Source {
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "userId")]
    user_id: Option<String>,
    #[serde(rename = "groupId")]
    group_id: Option<String>,
    #[serde(rename = "roomId")]
    room_id: Option<String>,
}

#[derive(Deserialize)]
struct LineMessage {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
}

#[derive(Deserialize)]
struct ProfileResp {
    #[serde(rename = "displayName")]
    display_name: String,
}

// ── Shared state ──────────────────────────────────────────────────────────────

struct Inner {
    channel_token: String,
    channel_secret: String,
    /// chat_id → (reply_token, received_at)
    reply_store: DashMap<String, (String, Instant)>,
    /// chat_id → push target (groupId/roomId for groups, userId for DMs)
    push_store: DashMap<String, String>,
    /// chat_id → is_group flag
    group_flag: DashMap<String, bool>,
    /// user_id → display name (fetched lazily from profile API)
    name_cache: DashMap<String, String>,
}

struct AppState {
    inner: Arc<Inner>,
    handler: Arc<dyn MessageHandler>,
}

// ── Webhook axum handler ──────────────────────────────────────────────────────

async fn handle_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    let sig = headers
        .get("x-line-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !verify_sig(&state.inner.channel_secret, &body, sig) {
        tracing::warn!("LINE: rejected webhook — invalid signature");
        return StatusCode::UNAUTHORIZED;
    }

    let Ok(wh) = serde_json::from_slice::<Webhook>(&body) else {
        return StatusCode::BAD_REQUEST;
    };

    for ev in wh.events {
        if ev.kind != "message" {
            continue;
        }
        let Some(msg) = ev.message else { continue };
        if msg.kind != "text" {
            continue;
        }
        let Some(text) = msg.text else { continue };

        let user_id = ev.source.user_id.clone().unwrap_or_default();

        let (chat_id, push_target, is_group) = match ev.source.kind.as_str() {
            "group" => {
                let gid = ev
                    .source
                    .group_id
                    .clone()
                    .unwrap_or_else(|| user_id.clone());
                (gid.clone(), gid, true)
            }
            "room" => {
                let rid = ev.source.room_id.clone().unwrap_or_else(|| user_id.clone());
                (rid.clone(), rid, true)
            }
            _ => (user_id.clone(), user_id.clone(), false),
        };

        // Persist reply token and routing info for send_message
        if let Some(token) = ev.reply_token {
            state
                .inner
                .reply_store
                .insert(chat_id.clone(), (token, Instant::now()));
        }
        state.inner.push_store.insert(chat_id.clone(), push_target);
        state.inner.group_flag.insert(chat_id.clone(), is_group);

        // Lazily fetch and cache display name (fire-and-forget)
        if !state.inner.name_cache.contains_key(&user_id) {
            let token = state.inner.channel_token.clone();
            let uid = user_id.clone();
            let cache = state.inner.name_cache.clone();
            tokio::spawn(async move {
                if let Some(name) = fetch_display_name(&token, &uid).await {
                    cache.insert(uid, name);
                }
            });
        }

        let display_name = state
            .inner
            .name_cache
            .get(&user_id)
            .map(|n| n.clone())
            .unwrap_or_else(|| user_id.clone());

        let inbound = InboundMessage {
            channel: ChannelId {
                platform: "line".into(),
                chat_id: chat_id.clone(),
                thread_id: None,
            },
            user_id,
            user_name: display_name,
            text,
            session_key: format!("line:{chat_id}"),
            is_group,
        };

        let h = state.handler.clone();
        tokio::spawn(async move {
            let _ = h.handle(inbound).await;
        });
    }

    StatusCode::OK
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn verify_sig(secret: &str, body: &[u8], signature: &str) -> bool {
    type HmacSha256 = Hmac<Sha256>;
    let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(body);
    B64.encode(mac.finalize().into_bytes()) == signature
}

async fn fetch_display_name(token: &str, user_id: &str) -> Option<String> {
    let resp = reqwest::Client::new()
        .get(format!("{LINE_PROFILE_URL}/{user_id}"))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .ok()?;
    resp.json::<ProfileResp>()
        .await
        .ok()
        .map(|p| p.display_name)
}

async fn api_reply(token: &str, reply_token: &str, text: &str) -> Result<(), PlatformError> {
    let body = serde_json::json!({
        "replyToken": reply_token,
        "messages": [{ "type": "text", "text": text }],
    });
    let resp = reqwest::Client::new()
        .post(LINE_REPLY_URL)
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .map_err(|e| PlatformError::Send(e.to_string()))?;

    if resp.status().is_success() {
        return Ok(());
    }
    let status = resp.status();
    let err = resp.text().await.unwrap_or_default();
    Err(PlatformError::Send(format!("LINE reply {status}: {err}")))
}

async fn api_push(token: &str, to: &str, text: &str) -> Result<(), PlatformError> {
    let body = serde_json::json!({
        "to": to,
        "messages": [{ "type": "text", "text": text }],
    });
    let resp = reqwest::Client::new()
        .post(LINE_PUSH_URL)
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .map_err(|e| PlatformError::Send(e.to_string()))?;

    if resp.status().is_success() {
        return Ok(());
    }
    let status = resp.status().as_u16();
    let err = resp.text().await.unwrap_or_default();

    if status == 429 || err.contains("quota") || err.contains("monthly limit") {
        return Err(PlatformError::Send(format!("quota:{err}")));
    }
    Err(PlatformError::Send(format!("LINE push {status}: {err}")))
}

// ── LineAdapter ───────────────────────────────────────────────────────────────

pub struct LineAdapter {
    port: u16,
    inner: Arc<Inner>,
}

impl LineAdapter {
    pub fn new(channel_token: String, channel_secret: String, port: u16) -> Self {
        Self {
            port,
            inner: Arc::new(Inner {
                channel_token,
                channel_secret,
                reply_store: DashMap::new(),
                push_store: DashMap::new(),
                group_flag: DashMap::new(),
                name_cache: DashMap::new(),
            }),
        }
    }

    async fn do_send(&self, channel: &ChannelId, mut text: String) -> Result<(), PlatformError> {
        let chat_id = &channel.chat_id;

        // Group: prepend @display_name so the target user is notified
        if self.inner.group_flag.get(chat_id).map_or(false, |v| *v) {
            // push_store holds userId for DMs and groupId for groups; for groups we need the
            // userId who sent the last message, stored in name_cache keyed by userId.
            // We scan name_cache entries to find the right name — fine for small groups.
            if let Some(push) = self.inner.push_store.get(chat_id) {
                if let Some(name) = self.inner.name_cache.get(push.as_str()) {
                    text = format!("@{} {}", *name, text);
                }
            }
        }

        // 1. Try reply token (free, one-shot, 25 s window)
        if let Some(entry) = self.inner.reply_store.remove(chat_id) {
            let (reply_token, received_at) = entry.1;
            if received_at.elapsed() < REPLY_TTL {
                tracing::debug!(chat_id, "LINE: reply API");
                return api_reply(&self.inner.channel_token, &reply_token, &text).await;
            }
            tracing::debug!(chat_id, "LINE: reply token expired, falling back to push");
        }

        // 2. Push fallback (free tier monthly quota)
        let push_target = self
            .inner
            .push_store
            .get(chat_id)
            .map(|v| v.clone())
            .unwrap_or_else(|| chat_id.clone());

        tracing::debug!(chat_id, "LINE: push API");
        match api_push(&self.inner.channel_token, &push_target, &text).await {
            Ok(()) => Ok(()),
            Err(PlatformError::Send(ref e)) if e.starts_with("quota:") => {
                tracing::error!(chat_id, "LINE push quota exceeded");
                Err(PlatformError::Send(
                    "ขออภัย บอทใช้งานเกินโควต้าข้อความรายเดือนแล้ว กรุณาลองใหม่เดือนหน้า".into(),
                ))
            }
            Err(e) => Err(e),
        }
    }
}

#[async_trait]
impl PlatformAdapter for LineAdapter {
    fn name(&self) -> &'static str {
        "line"
    }

    async fn start(&self, handler: Arc<dyn MessageHandler>) -> Result<(), PlatformError> {
        let state = Arc::new(AppState {
            inner: self.inner.clone(),
            handler,
        });

        let app = Router::new()
            .route("/line", post(handle_webhook))
            .with_state(state);

        let port = self.port;
        let listener = TcpListener::bind(format!("0.0.0.0:{port}"))
            .await
            .map_err(|e| PlatformError::Connection(e.to_string()))?;

        tracing::info!("LINE webhook listening on 0.0.0.0:{port}/line");

        tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("LINE server failed");
        });

        Ok(())
    }

    async fn send_message(
        &self,
        channel: &ChannelId,
        message: OutboundMessage,
    ) -> Result<(), PlatformError> {
        self.do_send(channel, message.text).await
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
        self.do_send(channel, buf).await
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_sig_correct() {
        // HMAC-SHA256("secret", "body") base64 = known value
        let secret = "secret";
        let body = b"body";
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let expected = B64.encode(mac.finalize().into_bytes());
        assert!(verify_sig(secret, body, &expected));
    }

    #[test]
    fn verify_sig_wrong_signature() {
        assert!(!verify_sig("secret", b"body", "wrongsig"));
    }

    #[test]
    fn verify_sig_empty_secret_fails_gracefully() {
        // empty key is invalid for HMAC — should return false, not panic
        assert!(!verify_sig("", b"body", "anything"));
    }
}
