use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use crate::{
    error::TransportError,
    types::{InferenceConfig, Message, StreamChunk, ToolSchema, TransportResponse},
};

#[derive(Debug, Clone, PartialEq)]
pub enum ApiMode {
    ChatCompletions,
    AnthropicMessages,
    CodexResponses,
    BedrockConverse,
}

pub type StreamResult = Pin<Box<dyn Stream<Item = Result<StreamChunk, TransportError>> + Send>>;

#[async_trait]
pub trait ProviderTransport: Send + Sync + 'static {
    fn api_mode(&self) -> ApiMode;

    async fn chat(
        &self,
        messages: &[Message],
        config: &InferenceConfig,
        tools: &[ToolSchema],
    ) -> Result<TransportResponse, TransportError>;

    async fn chat_stream(
        &self,
        messages: &[Message],
        config: &InferenceConfig,
        tools: &[ToolSchema],
    ) -> Result<StreamResult, TransportError>;
}
