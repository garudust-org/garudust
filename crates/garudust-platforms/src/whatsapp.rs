use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    body::Bytes,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Router,
};
use hmac::{Hmac, KeyInit, Mac};
use serde::Deserialize;
use serde_json::json;
use sha2::Sha256;
use tokio::net::TcpListener;

use futures::Stream;
use garudust_core::{
    error::PlatformError,
    platform::{MessageHandler, PlatformAdapter},
    types::{ChannelId, InboundMessage, OutboundMessage},
};

const WHATSAPP_API_URL: &str = "https://graph.facebook.com/v20.0";
/// WhatsApp Cloud API text message limit.
const WA_TEXT_LIMIT: usize = 4_096;

// ── WhatsApp Cloud API webhook deserialization ────────────────────────────────

#[derive(Deserialize)]
struct WebhookPayload {
    entry: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    changes: Vec<Change>,
}

#[derive(Deserialize)]
struct Change {
    value: ChangeValue,
}

#[derive(Deserialize)]
struct ChangeValue {
    contacts: Option<Vec<Contact>>,
    messages: Option<Vec<WaMessage>>,
}

#[derive(Deserialize)]
struct Contact {
    #[serde(rename = "wa_id")]
    wa_id: String,
    profile: Profile,
}

#[derive(Deserialize)]
struct Profile {
    name: String,
}

#[derive(Deserialize)]
struct WaMessage {
    from: String,
    #[serde(rename = "type")]
    kind: String,
    text: Option<WaText>,
}

#[derive(Deserialize)]
struct WaText {
    body: String,
}

// ── Webhook verification ──────────────────────────────────────────────────────

#[derive(Deserialize)]
struct VerifyParams {
    #[serde(rename = "hub.mode")]
    mode: String,
    #[serde(rename = "hub.verify_token")]
    verify_token: String,
    #[serde(rename = "hub.challenge")]
    challenge: String,
}

// ── Shared state ──────────────────────────────────────────────────────────────

struct Inner {
    access_token: String,
    phone_number_id: String,
    app_secret: String,
    verify_token: String,
    client: reqwest::Client,
}

struct WhatsAppState {
    inner: Arc<Inner>,
    handler: Arc<dyn MessageHandler>,
}

// ── Signature verification ────────────────────────────────────────────────────

fn verify_sig(app_secret: &str, body: &[u8], header: &str) -> bool {
    let expected = header.strip_prefix("sha256=").unwrap_or("");
    let Ok(expected_bytes) = hex::decode(expected) else {
        return false;
    };
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(app_secret.as_bytes()) else {
        return false;
    };
    mac.update(body);
    mac.verify_slice(&expected_bytes).is_ok()
}

// ── Text chunking ─────────────────────────────────────────────────────────────

fn chunk_text(text: &str) -> Vec<String> {
    if text.len() <= WA_TEXT_LIMIT {
        return vec![text.to_string()];
    }
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let end = (start + WA_TEXT_LIMIT).min(text.len());
        // Walk back to a char boundary
        let end = (start..=end)
            .rev()
            .find(|&i| text.is_char_boundary(i))
            .unwrap_or(end);
        chunks.push(text[start..end].to_string());
        start = end;
    }
    chunks
}

// ── Axum handlers ─────────────────────────────────────────────────────────────

async fn handle_verify(
    State(state): State<Arc<WhatsAppState>>,
    Query(params): Query<VerifyParams>,
) -> Result<String, StatusCode> {
    if params.mode == "subscribe" && params.verify_token == state.inner.verify_token {
        tracing::info!("WhatsApp: webhook verified");
        Ok(params.challenge)
    } else {
        tracing::warn!("WhatsApp: webhook verification failed — token mismatch");
        Err(StatusCode::FORBIDDEN)
    }
}

async fn handle_webhook(
    State(state): State<Arc<WhatsAppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    let sig = headers
        .get("x-hub-signature-256")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !state.inner.app_secret.is_empty() && !verify_sig(&state.inner.app_secret, &body, sig) {
        tracing::warn!("WhatsApp: rejected webhook — invalid signature");
        return StatusCode::UNAUTHORIZED;
    }

    let Ok(payload) = serde_json::from_slice::<WebhookPayload>(&body) else {
        return StatusCode::BAD_REQUEST;
    };

    for entry in payload.entry {
        for change in entry.changes {
            let value = change.value;
            let Some(messages) = value.messages else {
                continue;
            };
            let contacts = value.contacts.unwrap_or_default();

            for msg in messages {
                if msg.kind != "text" {
                    continue;
                }
                let Some(text_obj) = msg.text else { continue };
                let text = text_obj.body;
                let wa_id = msg.from.clone();

                let user_name = contacts
                    .iter()
                    .find(|c| c.wa_id == wa_id)
                    .map_or_else(|| wa_id.clone(), |c| c.profile.name.clone());

                let inbound = InboundMessage {
                    channel: ChannelId {
                        platform: "whatsapp".into(),
                        chat_id: wa_id.clone(),
                        thread_id: None,
                    },
                    user_id: wa_id.clone(),
                    user_name,
                    text,
                    session_key: format!("whatsapp:{wa_id}"),
                    is_group: false,
                };

                let handler = state.handler.clone();
                tokio::spawn(async move {
                    if let Err(e) = handler.handle(inbound).await {
                        tracing::error!(wa_id, "WhatsApp: handler error: {e}");
                    }
                });
            }
        }
    }

    StatusCode::OK
}

// ── Adapter ───────────────────────────────────────────────────────────────────

pub struct WhatsAppAdapter {
    port: u16,
    inner: Arc<Inner>,
}

impl WhatsAppAdapter {
    /// Create a new adapter.
    ///
    /// * `access_token`    — WhatsApp Cloud API access token
    /// * `phone_number_id` — Phone number ID from the Meta developer console
    /// * `app_secret`      — App secret for HMAC signature verification (pass empty string to skip)
    /// * `verify_token`    — Token used during webhook verification
    /// * `port`            — Local port to listen on for incoming webhooks
    pub fn new(
        access_token: String,
        phone_number_id: String,
        app_secret: String,
        verify_token: String,
        port: u16,
    ) -> Self {
        Self {
            port,
            inner: Arc::new(Inner {
                access_token,
                phone_number_id,
                app_secret,
                verify_token,
                client: reqwest::Client::new(),
            }),
        }
    }

    async fn do_send(&self, to: &str, text: &str) -> Result<(), PlatformError> {
        let url = format!("{WHATSAPP_API_URL}/{}/messages", self.inner.phone_number_id);

        for chunk in chunk_text(text) {
            let body = json!({
                "messaging_product": "whatsapp",
                "to": to,
                "type": "text",
                "text": { "body": chunk }
            });

            let resp = self
                .inner
                .client
                .post(&url)
                .bearer_auth(&self.inner.access_token)
                .json(&body)
                .send()
                .await
                .map_err(|e| PlatformError::Send(e.to_string()))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let detail = resp.text().await.unwrap_or_default();
                return Err(PlatformError::Send(format!(
                    "WhatsApp API error {status}: {detail}"
                )));
            }
        }

        Ok(())
    }
}

#[async_trait]
impl PlatformAdapter for WhatsAppAdapter {
    fn name(&self) -> &'static str {
        "whatsapp"
    }

    async fn start(&self, handler: Arc<dyn MessageHandler>) -> Result<(), PlatformError> {
        let state = Arc::new(WhatsAppState {
            inner: self.inner.clone(),
            handler,
        });

        let router = Router::new()
            .route("/whatsapp", get(handle_verify))
            .route("/whatsapp", post(handle_webhook))
            .with_state(state);

        let port = self.port;
        let listener = TcpListener::bind(format!("0.0.0.0:{port}"))
            .await
            .map_err(|e| PlatformError::Connection(e.to_string()))?;

        tracing::info!("WhatsApp adapter listening on 0.0.0.0:{port}");
        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, router).await {
                tracing::error!("WhatsApp server error: {e}");
            }
        });

        Ok(())
    }

    async fn send_message(
        &self,
        channel: &ChannelId,
        message: OutboundMessage,
    ) -> Result<(), PlatformError> {
        self.do_send(&channel.chat_id, &message.text).await
    }

    async fn send_stream(
        &self,
        channel: &ChannelId,
        mut stream: Pin<Box<dyn Stream<Item = String> + Send>>,
    ) -> Result<(), PlatformError> {
        use futures::StreamExt;
        let mut buf = String::new();
        while let Some(chunk) = stream.next().await {
            buf.push_str(&chunk);
        }
        self.send_message(channel, OutboundMessage::text(buf)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_short_text_unchanged() {
        let text = "hello";
        let chunks = chunk_text(text);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], text);
    }

    #[test]
    fn chunk_long_text_splits_on_char_boundary() {
        let text = "あ".repeat(2000); // each char is 3 bytes → 6000 bytes total
        let chunks = chunk_text(&text);
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.len() <= WA_TEXT_LIMIT);
        }
        assert_eq!(chunks.join(""), text);
    }

    #[test]
    fn verify_sig_rejects_bad_signature() {
        assert!(!verify_sig("secret", b"body", "sha256=badhex"));
    }

    #[test]
    fn verify_sig_accepts_correct_signature() {
        use hmac::Mac;
        let secret = "mysecret";
        let body = b"test body";
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let result = mac.finalize().into_bytes();
        let hex_sig = format!("sha256={}", hex::encode(result));
        assert!(verify_sig(secret, body, &hex_sig));
    }

    #[test]
    fn verify_sig_rejects_bad_hex_when_secret_nonempty() {
        assert!(!verify_sig("secret", b"body", "sha256=00000000"));
    }

    #[test]
    fn verify_sig_rejects_missing_prefix() {
        // Header must start with "sha256=" — bare hex must be rejected.
        use hmac::Mac;
        let secret = "s";
        let body = b"x";
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let bare_hex = hex::encode(mac.finalize().into_bytes());
        assert!(!verify_sig(secret, body, &bare_hex));
    }

    #[test]
    fn verify_sig_accepts_empty_body() {
        use hmac::Mac;
        let secret = "s";
        let body = b"";
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let sig = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));
        assert!(verify_sig(secret, body, &sig));
    }

    #[test]
    fn chunk_text_exactly_at_limit_is_single_chunk() {
        let text = "a".repeat(WA_TEXT_LIMIT);
        let chunks = chunk_text(&text);
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn chunk_text_one_over_limit_splits() {
        let text = "a".repeat(WA_TEXT_LIMIT + 1);
        let chunks = chunk_text(&text);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks.join(""), text);
    }
}
