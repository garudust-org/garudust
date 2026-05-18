use std::fmt::Write as _;
use std::path::Path;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};
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
use hmac::{Hmac, KeyInit, Mac};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;
use tokio::sync::Mutex as AsyncMutex;

use garudust_core::{
    error::PlatformError,
    platform::{MessageHandler, PlatformAdapter},
    types::{ChannelId, DocAttachment, ImageAttachment, InboundMessage, OutboundMessage},
};
use garudust_memory::BotIdentityStore;

const LINE_REPLY_URL: &str = "https://api.line.me/v2/bot/message/reply";
const LINE_PUSH_URL: &str = "https://api.line.me/v2/bot/message/push";
const LINE_PROFILE_URL: &str = "https://api.line.me/v2/bot/profile";
const LINE_BOT_INFO_URL: &str = "https://api.line.me/v2/bot/info";
const LINE_CONTENT_URL: &str = "https://api-data.line.me/v2/bot/message";
/// Reply token is valid for 30 s; leave a 5 s safety margin.
const REPLY_TTL: Duration = Duration::from_secs(25);
const LINE_TEXT_LIMIT: usize = 5_000;
/// Evict name_cache once it grows beyond this; names are cheap to re-fetch.
const MAX_CACHE_ENTRIES: usize = 50_000;
/// Max attempts to fetch /v2/bot/info at startup before giving up (lazy
/// webhook re-fetch then takes over). Backoff doubles from 1s, capped at 16s.
const BOT_INFO_RETRIES: u32 = 5;
/// Minimum spacing between lazy /v2/bot/info re-fetch attempts triggered from
/// the webhook path, so a sustained outage + busy group cannot hammer the API.
const BOT_INFO_RELOOKUP_INTERVAL: Duration = Duration::from_secs(60);

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
    id: String,
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
    /// Original file name for `type = "file"` messages.
    #[serde(rename = "fileName")]
    file_name: Option<String>,
    /// Structured mention info delivered by LINE when users tag others in groups.
    mention: Option<Mention>,
}

#[derive(Deserialize)]
struct Mention {
    #[serde(default)]
    mentionees: Vec<Mentionee>,
}

#[derive(Deserialize)]
struct Mentionee {
    #[serde(rename = "userId")]
    user_id: Option<String>,
}

#[derive(Deserialize)]
struct ProfileResp {
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Deserialize)]
struct BotInfoResp {
    #[serde(rename = "userId")]
    user_id: String,
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
    /// Bot's own LINE userId, fetched once at start via `/v2/bot/info`.
    /// Used to detect mentions of the bot from `event.message.mention.mentionees`.
    /// Empty when the fetch failed — gateway will fall back to text-contains matching.
    bot_self_user_id: OnceLock<String>,
    /// sha256(channel_token) — cache key for the bot userId in state.db.
    token_hash: String,
    /// Persistent bot-userId cache. `None` if the DB could not be opened.
    bot_id_store: Option<BotIdentityStore>,
    /// Single-flight + throttle gate for lazy /bot/info re-fetch. Holds the
    /// instant of the last attempt; `try_lock` failure means a probe is
    /// already in flight, so callers skip.
    botinfo_gate: AsyncMutex<Option<Instant>>,
}

impl Inner {
    /// Record the resolved bot userId into the in-memory `OnceLock` and, on
    /// first set, persist it to state.db so future restarts skip the network.
    fn remember_bot_id(&self, user_id: &str) {
        if self.bot_self_user_id.set(user_id.to_owned()).is_ok() {
            if let Some(store) = &self.bot_id_store {
                if let Err(e) = store.put(&self.token_hash, user_id) {
                    tracing::warn!(error = %e, "LINE: failed to persist bot userId cache");
                } else {
                    tracing::debug!("LINE: bot userId persisted to state.db");
                }
            }
        }
    }

    /// Lazy, throttled, single-flight re-fetch of the bot userId. Used from the
    /// webhook path to self-heal when the startup fetch failed, without needing
    /// a restart and without hammering the API during a sustained outage.
    async fn ensure_bot_id(&self) {
        if self.bot_self_user_id.get().is_some() {
            return;
        }
        // Single-flight: only one probe runs at a time; concurrent callers bail.
        let Ok(mut gate) = self.botinfo_gate.try_lock() else {
            return;
        };
        // Re-check: another probe may have resolved it while we waited.
        if self.bot_self_user_id.get().is_some() {
            return;
        }
        if let Some(last) = *gate {
            if last.elapsed() < BOT_INFO_RELOOKUP_INTERVAL {
                return;
            }
        }
        *gate = Some(Instant::now());
        if let Some(uid) = fetch_bot_info(&self.client, &self.channel_token).await {
            tracing::info!("LINE: bot self userId fetched lazily — mention detection active");
            self.remember_bot_id(&uid);
        }
    }
}

/// Fetch the bot's own userId from LINE `/v2/bot/info`. Returns `None` on any
/// network / parse / non-success failure; the caller decides whether to retry.
async fn fetch_bot_info(client: &reqwest::Client, token: &str) -> Option<String> {
    match client
        .get(LINE_BOT_INFO_URL)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(info) = resp.json::<BotInfoResp>().await {
                Some(info.user_id)
            } else {
                tracing::warn!("LINE: /bot/info response did not parse");
                None
            }
        }
        Ok(resp) => {
            tracing::warn!(status = %resp.status(), "LINE: /bot/info returned non-success");
            None
        }
        Err(e) => {
            tracing::warn!(error = %e, "LINE: failed to fetch /bot/info");
            None
        }
    }
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
        if msg.kind != "text" && msg.kind != "image" && msg.kind != "file" {
            continue;
        }

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

        // Fetch the display name synchronously so it is available for this
        // message. In group/room sources the 1-on-1 profile endpoint 404s for
        // non-friends; we use the scoped member endpoint instead.
        let display_name = if let Some(cached) = state.inner.name_cache.get(&user_id) {
            cached.clone()
        } else {
            let scope = match ev.source.kind.as_str() {
                "group" => ProfileScope::Group(&chat_id),
                "room" => ProfileScope::Room(&chat_id),
                _ => ProfileScope::Personal,
            };
            let name = fetch_display_name(
                &state.inner.client,
                &state.inner.channel_token,
                &user_id,
                scope,
            )
            .await
            .unwrap_or_else(|| user_id.clone());
            state.inner.name_cache.insert(user_id.clone(), name.clone());
            name
        };

        // Self-heal: if the startup fetch failed, lazily (throttled,
        // single-flight) re-resolve the bot userId before deciding mentions.
        if is_group {
            state.inner.ensure_bot_id().await;
        }

        // Structured mention detection: cross-reference message.mention.mentionees
        // against the bot's own userId fetched at start. Only meaningful in
        // groups; for DMs the gateway ignores `bot_mentioned`.
        let bot_mentioned = if is_group {
            state.inner.bot_self_user_id.get().map(|self_id| {
                msg.mention.as_ref().is_some_and(|m| {
                    m.mentionees
                        .iter()
                        .any(|x| x.user_id.as_deref() == Some(self_id.as_str()))
                })
            })
        } else {
            None
        };

        // Download image or file content from LINE Content API.
        let mut text = String::new();
        let mut attachments: Vec<ImageAttachment> = Vec::new();
        let mut doc_attachments: Vec<DocAttachment> = Vec::new();

        match msg.kind.as_str() {
            "image" => {
                let msg_id = msg.id.clone();
                let token = state.inner.channel_token.clone();
                let client = state.inner.client.clone();
                let path = format!("/tmp/garudust_line_{msg_id}.jpg");
                match download_line_content(&client, &token, &msg_id, &path).await {
                    Ok(()) => attachments.push(ImageAttachment { path }),
                    Err(e) => {
                        tracing::warn!(msg_id, error = %e, "LINE: image download failed");
                    }
                }
            }
            "file" => {
                let msg_id = msg.id.clone();
                let file_name = msg
                    .file_name
                    .clone()
                    .unwrap_or_else(|| format!("{msg_id}.bin"));
                let ext = std::path::Path::new(&file_name)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("bin");
                if is_supported_doc_ext(ext) {
                    let token = state.inner.channel_token.clone();
                    let client = state.inner.client.clone();
                    let path = format!("/tmp/garudust_line_{msg_id}.{ext}");
                    match download_line_content(&client, &token, &msg_id, &path).await {
                        Ok(()) => doc_attachments.push(DocAttachment { path, file_name }),
                        Err(e) => {
                            tracing::warn!(msg_id, error = %e, "LINE: file download failed");
                        }
                    }
                }
            }
            _ => {
                text = msg.text.unwrap_or_default();
            }
        }

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
            bot_mentioned,
            attachments,
            doc_attachments,
        };

        let h = state.handler.clone();
        tokio::spawn(async move {
            let _ = h.handle(inbound).await;
        });
    }

    StatusCode::OK
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn download_line_content(
    client: &reqwest::Client,
    token: &str,
    message_id: &str,
    dest: &str,
) -> Result<(), PlatformError> {
    let url = format!("{LINE_CONTENT_URL}/{message_id}/content");
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| PlatformError::Send(e.to_string()))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(PlatformError::Send(format!(
            "LINE content {status}: {body}"
        )));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| PlatformError::Send(e.to_string()))?;
    tokio::fs::write(dest, &bytes)
        .await
        .map_err(|e| PlatformError::Send(e.to_string()))
}

fn is_supported_doc_ext(ext: &str) -> bool {
    matches!(
        ext.to_lowercase().as_str(),
        "pdf" | "txt" | "csv" | "md" | "json" | "docx" | "doc" | "xlsx" | "xls"
    )
}

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

/// Scope for the profile lookup. The 1-on-1 `/v2/bot/profile/{userId}`
/// endpoint returns 404 for group members who haven't added the bot as a
/// friend, so for group/room sources we use the scoped member endpoint
/// which works for any member regardless of friend status.
enum ProfileScope<'a> {
    Personal,
    Group(&'a str),
    Room(&'a str),
}

async fn fetch_display_name(
    client: &reqwest::Client,
    token: &str,
    user_id: &str,
    scope: ProfileScope<'_>,
) -> Option<String> {
    let (url, scope_label) = match scope {
        ProfileScope::Personal => (format!("{LINE_PROFILE_URL}/{user_id}"), "personal"),
        ProfileScope::Group(gid) => (
            format!("https://api.line.me/v2/bot/group/{gid}/member/{user_id}"),
            "group",
        ),
        ProfileScope::Room(rid) => (
            format!("https://api.line.me/v2/bot/room/{rid}/member/{user_id}"),
            "room",
        ),
    };
    let resp = match client
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(scope = scope_label, user_id, error = %e, "LINE: profile fetch network error");
            return None;
        }
    };
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        tracing::warn!(
            scope = scope_label,
            user_id,
            status = status.as_u16(),
            body = %body,
            "LINE: profile fetch returned non-success — falling back to userId"
        );
        return None;
    }
    match resp.json::<ProfileResp>().await {
        Ok(p) => Some(p.display_name),
        Err(e) => {
            tracing::warn!(scope = scope_label, user_id, error = %e, "LINE: profile response failed to parse");
            None
        }
    }
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
    webhook_path: String,
    inner: Arc<Inner>,
}

impl LineAdapter {
    pub fn new(
        channel_token: String,
        channel_secret: String,
        port: u16,
        webhook_path: String,
        home_dir: &Path,
    ) -> Self {
        let token_hash =
            Sha256::digest(channel_token.as_bytes())
                .iter()
                .fold(String::new(), |mut acc, b| {
                    let _ = write!(acc, "{b:02x}");
                    acc
                });
        let bot_id_store = match BotIdentityStore::open(home_dir) {
            Ok(s) => Some(s),
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "LINE: bot identity cache unavailable — relying on API + lazy re-fetch"
                );
                None
            }
        };
        Self {
            port,
            webhook_path,
            inner: Arc::new(Inner {
                channel_token,
                channel_secret,
                client: reqwest::Client::new(),
                reply_store: DashMap::new(),
                push_store: DashMap::new(),
                group_flag: DashMap::new(),
                last_sender: DashMap::new(),
                name_cache: DashMap::new(),
                bot_self_user_id: OnceLock::new(),
                token_hash,
                bot_id_store,
                botinfo_gate: AsyncMutex::new(None),
            }),
        }
    }

    async fn do_send(&self, channel: &ChannelId, mut text: String) -> Result<(), PlatformError> {
        let chat_id = &channel.chat_id;

        text = truncate_to_line_limit(text);

        // Prepend @mention in group chats; name_cache is populated before InboundMessage is built
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
        // Resolve the bot's own userId for @mention detection against LINE's
        // structured `mention.mentionees`.
        //
        // 1. Prefer the persisted cache (state.db): a token's userId is
        //    immutable, so after the first successful fetch ever no network is
        //    needed at startup — a restart-time blip can't disable detection.
        // 2. On a cache miss, fetch /v2/bot/info in the background with bounded
        //    retry+backoff (so the webhook listener binds immediately).
        // 3. If every retry fails, it stays unset; the webhook path lazily
        //    re-fetches (throttled) and the gateway falls back to text match.
        if let Some(uid) = self
            .inner
            .bot_id_store
            .as_ref()
            .and_then(|s| s.get(&self.inner.token_hash))
        {
            let _ = self.inner.bot_self_user_id.set(uid);
            tracing::info!("LINE: bot self userId loaded from cache — mention detection active");
        } else {
            let inner = self.inner.clone();
            tokio::spawn(async move {
                let mut delay = Duration::from_secs(1);
                for attempt in 1..=BOT_INFO_RETRIES {
                    if let Some(uid) = fetch_bot_info(&inner.client, &inner.channel_token).await {
                        tracing::info!("LINE: bot self userId fetched — mention detection active");
                        inner.remember_bot_id(&uid);
                        return;
                    }
                    if attempt < BOT_INFO_RETRIES {
                        tracing::warn!(
                            attempt,
                            retry_in_secs = delay.as_secs(),
                            "LINE: /bot/info fetch failed — retrying"
                        );
                        tokio::time::sleep(delay).await;
                        delay = (delay * 2).min(Duration::from_secs(16));
                    }
                }
                tracing::warn!(
                    "LINE: /bot/info failed after {BOT_INFO_RETRIES} attempts — mention \
                     detection disabled until a later webhook re-fetch or restart"
                );
            });
        }

        let state = Arc::new(LineState {
            inner: self.inner.clone(),
            handler,
        });

        let app = Router::new()
            .route(&self.webhook_path, post(handle_webhook))
            .with_state(state);

        let port = self.port;
        let path = self.webhook_path.clone();
        let listener = TcpListener::bind(format!("0.0.0.0:{port}"))
            .await
            .map_err(|e| PlatformError::Connection(e.to_string()))?;

        tracing::info!("LINE webhook listening on 0.0.0.0:{port}{path}");

        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                tracing::error!("LINE server exited: {e}");
            }
        });

        // Periodic eviction: prune expired reply tokens every 60 s; clear name
        // cache when it exceeds MAX_CACHE_ENTRIES (names are cheap to re-fetch).
        let inner_gc = self.inner.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
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
