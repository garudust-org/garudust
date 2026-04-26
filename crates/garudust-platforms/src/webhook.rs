use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use axum::{extract::State, routing::post, Json, Router};
use futures::Stream;
use garudust_core::{
    error::PlatformError,
    platform::{MessageHandler, PlatformAdapter},
    types::{ChannelId, InboundMessage, OutboundMessage},
};
use serde::{Deserialize, Serialize};

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

async fn handle_webhook(
    State(handler): State<Arc<dyn MessageHandler>>,
    Json(payload): Json<WebhookPayload>,
) -> axum::http::StatusCode {
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
    };

    match handler.handle(inbound).await {
        Ok(()) => axum::http::StatusCode::ACCEPTED,
        Err(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
    }
}

pub struct WebhookAdapter {
    port: u16,
}

impl WebhookAdapter {
    pub fn new(port: u16) -> Self {
        Self { port }
    }
}

#[async_trait]
impl PlatformAdapter for WebhookAdapter {
    fn name(&self) -> &'static str {
        "webhook"
    }

    async fn start(&self, handler: Arc<dyn MessageHandler>) -> Result<(), PlatformError> {
        let port = self.port;
        let router = Router::new()
            .route("/webhook", post(handle_webhook))
            .with_state(handler);

        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
            .await
            .map_err(|e| PlatformError::Connection(e.to_string()))?;

        tracing::info!("webhook adapter listening on 0.0.0.0:{port}");
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
