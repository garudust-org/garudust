use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::Stream;
use garudust_core::{
    error::PlatformError,
    platform::{MessageHandler, PlatformAdapter},
    types::{ChannelId, OutboundMessage},
};

/// Minimal webhook adapter — receives POST requests and sends responses via HTTP callback.
pub struct WebhookAdapter;

#[async_trait]
impl PlatformAdapter for WebhookAdapter {
    fn name(&self) -> &'static str {
        "webhook"
    }

    async fn start(&self, _handler: Arc<dyn MessageHandler>) -> Result<(), PlatformError> {
        // TODO: start axum listener
        Ok(())
    }

    async fn send_message(
        &self,
        _channel: &ChannelId,
        _message: OutboundMessage,
    ) -> Result<(), PlatformError> {
        // TODO: HTTP POST to callback URL
        Ok(())
    }

    async fn send_stream(
        &self,
        _channel: &ChannelId,
        _stream: Pin<Box<dyn Stream<Item = String> + Send>>,
    ) -> Result<(), PlatformError> {
        Ok(())
    }
}
