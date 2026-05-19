use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use axum::{extract::State, http::HeaderMap, routing::post, Router};
use bytes::Bytes;
use futures::Stream;
use garudust_core::{
    error::PlatformError,
    net_guard,
    platform::{MessageHandler, PlatformAdapter},
    types::{ChannelId, InboundMessage, OutboundMessage},
};
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[derive(Deserialize)]
struct WebhookPayload {
    text: String,
    /// URL to POST the response back to.
    callback_url: String,
    #[serde(default)]
    user_id: String,
    #[serde(default)]
    user_name: String,
    #[serde(default)]
    session_key: String,
}

#[derive(Serialize)]
struct CallbackPayload {
    text: String,
}

struct WebhookState {
    handler: Arc<dyn MessageHandler>,
    hmac_secret: Option<String>,
}

async fn handle_webhook(
    State(state): State<Arc<WebhookState>>,
    headers: HeaderMap,
    body: Bytes,
) -> axum::http::StatusCode {
    if let Some(secret) = &state.hmac_secret {
        let sig = headers
            .get("x-hub-signature-256")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !verify_sig(secret, &body, sig) {
            tracing::warn!("webhook: rejected request — invalid HMAC signature");
            return axum::http::StatusCode::UNAUTHORIZED;
        }
    }

    let Ok(payload) = serde_json::from_slice::<WebhookPayload>(&body) else {
        return axum::http::StatusCode::BAD_REQUEST;
    };

    let session_key = if payload.session_key.is_empty() {
        format!("webhook:{}", payload.callback_url)
    } else {
        payload.session_key.clone()
    };

    let inbound = InboundMessage {
        channel: ChannelId {
            platform: "webhook".into(),
            // chat_id holds the callback URL so send_message can POST back
            chat_id: payload.callback_url,
            thread_id: None,
        },
        user_id: payload.user_id,
        user_name: payload.user_name,
        text: payload.text,
        session_key,
        is_group: false,
        bot_mentioned: None,
        attachments: vec![],
        doc_attachments: vec![],
    };

    match state.handler.handle(inbound).await {
        Ok(()) => axum::http::StatusCode::ACCEPTED,
        Err(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// Verify `X-Hub-Signature-256: sha256=<hex>` against HMAC-SHA256(secret, body).
/// Uses constant-time comparison to prevent timing attacks.
fn verify_sig(secret: &str, body: &[u8], signature: &str) -> bool {
    let hex_sig = signature.strip_prefix("sha256=").unwrap_or("");
    let Ok(sig_bytes) = hex::decode(hex_sig) else {
        return false;
    };
    let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(body);
    mac.verify_slice(&sig_bytes).is_ok()
}

pub struct WebhookAdapter {
    port: u16,
    webhook_path: String,
    hmac_secret: Option<String>,
}

impl WebhookAdapter {
    pub fn new(port: u16, webhook_path: String, hmac_secret: Option<String>) -> Self {
        Self {
            port,
            webhook_path,
            hmac_secret,
        }
    }
}

#[async_trait]
impl PlatformAdapter for WebhookAdapter {
    fn name(&self) -> &'static str {
        "webhook"
    }

    async fn start(&self, handler: Arc<dyn MessageHandler>) -> Result<(), PlatformError> {
        let port = self.port;
        let path = self.webhook_path.clone();
        let state = Arc::new(WebhookState {
            handler,
            hmac_secret: self.hmac_secret.clone(),
        });
        let router = Router::new()
            .route(&self.webhook_path, post(handle_webhook))
            .with_state(state);

        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
            .await
            .map_err(|e| PlatformError::Connection(e.to_string()))?;

        tracing::info!("webhook adapter listening on 0.0.0.0:{port}{path}");
        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, router).await {
                tracing::error!("webhook server error: {e}");
            }
        });
        Ok(())
    }

    async fn send_message(
        &self,
        channel: &ChannelId,
        message: OutboundMessage,
    ) -> Result<(), PlatformError> {
        net_guard::is_safe_url(&channel.chat_id).map_err(|e| PlatformError::Send(e.to_string()))?;

        let client = reqwest::Client::new();
        client
            .post(&channel.chat_id)
            .json(&CallbackPayload { text: message.text })
            .send()
            .await
            .map_err(|e| PlatformError::Send(e.to_string()))?;
        Ok(())
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
    use garudust_core::net_guard;

    #[test]
    fn send_message_rejects_private_callback_url() {
        let result = net_guard::is_safe_url("http://192.168.1.1/callback");
        assert!(result.is_err(), "private callback URL must be blocked");
    }

    #[test]
    fn session_key_falls_back_to_callback_url() {
        let session_key = "";
        let callback_url = "https://example.com/reply";
        let key = if session_key.is_empty() {
            format!("webhook:{callback_url}")
        } else {
            session_key.to_string()
        };
        assert_eq!(key, "webhook:https://example.com/reply");
    }

    #[test]
    fn session_key_used_when_provided() {
        let session_key = "my-custom-key";
        let callback_url = "https://example.com/reply";
        let key = if session_key.is_empty() {
            format!("webhook:{callback_url}")
        } else {
            session_key.to_string()
        };
        assert_eq!(key, "my-custom-key");
    }

    // ── verify_sig ────────────────────────────────────────────────────────────

    fn make_sig(secret: &str, body: &[u8]) -> String {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
    }

    #[test]
    fn verify_sig_correct() {
        let secret = "my-secret";
        let body = b"hello world";
        let sig = make_sig(secret, body);
        assert!(verify_sig(secret, body, &sig));
    }

    #[test]
    fn verify_sig_wrong_secret() {
        let body = b"hello world";
        let sig = make_sig("correct-secret", body);
        assert!(!verify_sig("wrong-secret", body, &sig));
    }

    #[test]
    fn verify_sig_tampered_body() {
        let secret = "my-secret";
        let sig = make_sig(secret, b"original body");
        assert!(!verify_sig(secret, b"tampered body", &sig));
    }

    #[test]
    fn verify_sig_missing_prefix_rejected() {
        let secret = "my-secret";
        let body = b"body";
        // Raw hex without "sha256=" prefix
        let raw_hex = hex::encode({
            let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
            mac.update(body);
            mac.finalize().into_bytes()
        });
        assert!(!verify_sig(secret, body, &raw_hex));
    }

    #[test]
    fn verify_sig_empty_signature_rejected() {
        assert!(!verify_sig("secret", b"body", ""));
    }
}
