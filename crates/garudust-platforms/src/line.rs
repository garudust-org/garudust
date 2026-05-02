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
use dashmap::{DashMap, DashSet};
use futures::{Stream, StreamExt};
use hmac::{Hmac, KeyInit, Mac};
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
const LINE_TEXT_LIMIT: usize = 5_000;
/// Evict name_cache once it grows beyond this; names are cheap to re-fetch.
const MAX_CACHE_ENTRIES: usize = 50_000;

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

// ── Error type for push API results ──────────────────────────────────────────

enum PushOutcome {
    Ok,
    QuotaExceeded,
    Err(PlatformError),
}

// ── Shared state ──────────────────────────────────────────────────────────────

struct Inner {
    channel_token: String,
    channel_secret: String,
    client: reqwest::Client,
    /// chat_id → (reply_token, received_at)
    reply_store: DashMap<String, (String, Instant)>,
    /// chat_id → push target (groupId/roomId for groups, userId for DMs)
    push_store: DashMap<String, String>,
    /// chat_id → is_group flag
    group_flag: DashMap<String, bool>,
    /// chat_id → last sender's user_id (used for @mention in groups)
    last_sender: DashMap<String, String>,
    /// user_id → display name (fetched lazily from profile API)
    name_cache: DashMap<String, String>,
    /// user_ids currently being fetched — prevents redundant profile API calls
    fetching: DashSet<String>,
}

struct LineState {
    inner: Arc<Inner>,
    handler: Arc<dyn MessageHandler>,
}

// ── Webhook axum handler ──────────────────────────────────────────────────────

async fn handle_webhook(
    State(state): State<Arc<LineState>>,
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

        // Events without userId (e.g. some bot-event types) are unusable
        let Some(user_id) = ev.source.user_id.clone() else {
            continue;
        };

        let (chat_id, push_target, is_group) = match ev.source.kind.as_str() {
            "group" => {
                let Some(gid) = ev.source.group_id.clone() else {
                    tracing::warn!("LINE: group event missing groupId — skipping");
                    continue;
                };
                (gid.clone(), gid, true)
            }
            "room" => {
                let Some(rid) = ev.source.room_id.clone() else {
                    tracing::warn!("LINE: room event missing roomId — skipping");
                    continue;
                };
                (rid.clone(), rid, true)
            }
            _ => (user_id.clone(), user_id.clone(), false),
        };

        if let Some(token) = ev.reply_token {
            state
                .inner
                .reply_store
                .insert(chat_id.clone(), (token, Instant::now()));
        }
        state.inner.push_store.insert(chat_id.clone(), push_target);
        state.inner.group_flag.insert(chat_id.clone(), is_group);
        state
            .inner
            .last_sender
            .insert(chat_id.clone(), user_id.clone());

        // Deduplicated lazy profile fetch: fetching set ensures only one in-flight
        // request per user even under concurrent webhook events.
        if !state.inner.name_cache.contains_key(&user_id)
            && state.inner.fetching.insert(user_id.clone())
        {
            let token = state.inner.channel_token.clone();
            let uid = user_id.clone();
            let cache = state.inner.name_cache.clone();
            let in_flight = state.inner.fetching.clone();
            let client = state.inner.client.clone();
            tokio::spawn(async move {
                if let Some(name) = fetch_display_name(&client, &token, &uid).await {
                    cache.insert(uid.clone(), name);
                }
                in_flight.remove(&uid);
            });
        }

        let display_name = state
            .inner
            .name_cache
            .get(&user_id)
            .map_or_else(|| user_id.clone(), |n| n.clone());

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
    // Decode first so a malformed/short sig returns false before touching HMAC state.
    let Ok(sig_bytes) = B64.decode(signature) else {
        return false;
    };
    let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(body);
    // verify_slice is constant-time; plain string == leaks timing info.
    mac.verify_slice(&sig_bytes).is_ok()
}

fn truncate_to_line_limit(text: String) -> String {
    if text.chars().count() <= LINE_TEXT_LIMIT {
        return text;
    }
    let suffix = "… [ข้อความถูกตัดให้อยู่ในขีดจำกัดของ LINE]";
    let keep = LINE_TEXT_LIMIT.saturating_sub(suffix.chars().count());
    let truncated: String = text.chars().take(keep).collect();
    format!("{truncated}{suffix}")
}

async fn fetch_display_name(
    client: &reqwest::Client,
    token: &str,
    user_id: &str,
) -> Option<String> {
    let resp = client
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

async fn api_reply(
    client: &reqwest::Client,
    token: &str,
    reply_token: &str,
    text: &str,
) -> Result<(), PlatformError> {
    let body = serde_json::json!({
        "replyToken": reply_token,
        "messages": [{ "type": "text", "text": text }],
    });
    let resp = client
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

async fn api_push(client: &reqwest::Client, token: &str, to: &str, text: &str) -> PushOutcome {
    let body = serde_json::json!({
        "to": to,
        "messages": [{ "type": "text", "text": text }],
    });
    let resp = match client
        .post(LINE_PUSH_URL)
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return PushOutcome::Err(PlatformError::Send(e.to_string())),
    };

    if resp.status().is_success() {
        return PushOutcome::Ok;
    }
    let status = resp.status().as_u16();
    let err = resp.text().await.unwrap_or_default();

    if status == 429 {
        return PushOutcome::QuotaExceeded;
    }
    PushOutcome::Err(PlatformError::Send(format!("LINE push {status}: {err}")))
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
                client: reqwest::Client::new(),
                reply_store: DashMap::new(),
                push_store: DashMap::new(),
                group_flag: DashMap::new(),
                last_sender: DashMap::new(),
                name_cache: DashMap::new(),
                fetching: DashSet::new(),
            }),
        }
    }

    async fn do_send(&self, channel: &ChannelId, mut text: String) -> Result<(), PlatformError> {
        let chat_id = &channel.chat_id;

        text = truncate_to_line_limit(text);

        // Prepend @mention in group chats; fall back to @user_id while name is being fetched
        if self.inner.group_flag.get(chat_id).is_some_and(|v| *v) {
            if let Some(uid) = self.inner.last_sender.get(chat_id) {
                let mention = self
                    .inner
                    .name_cache
                    .get(uid.as_str())
                    .map_or_else(|| uid.clone(), |n| n.clone());
                text = format!("@{mention} {text}");
            }
        }

        // Reply API first (free, one-shot, 25 s window). On transient failure we
        // fall through to push rather than dropping the message entirely.
        if let Some(entry) = self.inner.reply_store.remove(chat_id) {
            let (reply_token, received_at) = entry.1;
            if received_at.elapsed() < REPLY_TTL {
                tracing::debug!(chat_id, "LINE: reply API");
                if api_reply(
                    &self.inner.client,
                    &self.inner.channel_token,
                    &reply_token,
                    &text,
                )
                .await
                .is_ok()
                {
                    return Ok(());
                }
                tracing::warn!(chat_id, "LINE: reply API failed, falling back to push");
            } else {
                tracing::debug!(chat_id, "LINE: reply token expired, falling back to push");
            }
        }

        // Push fallback (free tier monthly quota)
        let push_target = self
            .inner
            .push_store
            .get(chat_id)
            .map_or_else(|| chat_id.clone(), |v| v.clone());

        tracing::debug!(chat_id, "LINE: push API");
        match api_push(
            &self.inner.client,
            &self.inner.channel_token,
            &push_target,
            &text,
        )
        .await
        {
            PushOutcome::Ok => Ok(()),
            PushOutcome::QuotaExceeded => {
                tracing::error!(chat_id, "LINE push quota exceeded");
                Err(PlatformError::Send(
                    "ขออภัย บอทใช้งานเกินโควต้าข้อความรายเดือนแล้ว กรุณาลองใหม่เดือนหน้า".into(),
                ))
            }
            PushOutcome::Err(e) => Err(e),
        }
    }
}

#[async_trait]
impl PlatformAdapter for LineAdapter {
    fn name(&self) -> &'static str {
        "line"
    }

    async fn start(&self, handler: Arc<dyn MessageHandler>) -> Result<(), PlatformError> {
        let state = Arc::new(LineState {
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
            if let Err(e) = axum::serve(listener, app).await {
                tracing::error!("LINE server exited: {e}");
            }
        });

        // Periodic eviction: prune expired reply tokens every 60 s; clear name
        // cache when it exceeds MAX_CACHE_ENTRIES (names are cheap to re-fetch).
        let inner_gc = self.inner.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_mins(1));
            loop {
                interval.tick().await;
                inner_gc
                    .reply_store
                    .retain(|_, (_, t)| t.elapsed() < REPLY_TTL);
                if inner_gc.name_cache.len() > MAX_CACHE_ENTRIES {
                    inner_gc.name_cache.clear();
                }
            }
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
        type HmacSha256 = Hmac<Sha256>;
        let secret = "secret";
        let body = b"body";
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
    fn verify_sig_empty_secret() {
        // hmac accepts empty keys; the Base64-decoded bytes simply won't match the HMAC
        assert!(!verify_sig("", b"body", "anything"));
    }

    #[test]
    fn truncate_short_text_unchanged() {
        let s = "สวัสดี".to_string();
        assert_eq!(truncate_to_line_limit(s.clone()), s);
    }

    #[test]
    fn truncate_long_text_fits_limit() {
        let long: String = "a".repeat(6_000);
        let result = truncate_to_line_limit(long);
        assert!(result.chars().count() <= LINE_TEXT_LIMIT);
        assert!(result.contains("LINE"));
    }
}
