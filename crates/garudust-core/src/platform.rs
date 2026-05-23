use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::Stream;

use crate::{
    error::PlatformError,
    types::{ChannelId, InboundMessage, OutboundMessage},
};

#[async_trait]
pub trait PlatformAdapter: Send + Sync + 'static {
    fn name(&self) -> &'static str;

    async fn start(&self, handler: Arc<dyn MessageHandler>) -> Result<(), PlatformError>;

    /// Signal the platform's background task to stop. Default no-op for adapters
    /// that do not support clean cancellation (e.g. axum-based HTTP servers that
    /// rely on process exit). Adapters that own a JoinHandle should abort it here.
    async fn stop(&self) {}

    /// Liveness probe called by the `/health` endpoint. Return `Ok(())` when the
    /// adapter is operating normally, or an error string describing the fault.
    /// Default implementation always returns `Ok(())` — adapters may override this
    /// to ping their upstream API or check their internal connection state.
    async fn health_check(&self) -> Result<(), String> {
        Ok(())
    }

    async fn send_message(
        &self,
        channel: &ChannelId,
        message: OutboundMessage,
    ) -> Result<(), PlatformError>;

    async fn send_stream(
        &self,
        channel: &ChannelId,
        stream: Pin<Box<dyn Stream<Item = String> + Send>>,
    ) -> Result<(), PlatformError>;
}

#[async_trait]
pub trait MessageHandler: Send + Sync + 'static {
    async fn handle(&self, msg: InboundMessage) -> Result<(), anyhow::Error>;
}
