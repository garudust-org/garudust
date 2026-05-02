#![cfg(test)]

use std::sync::Arc;

use async_trait::async_trait;
use garudust_core::{
    config::AgentConfig,
    error::{AgentError, TransportError},
    memory::{MemoryContent, MemoryStore},
    tool::{ApprovalDecision, CommandApprover},
    transport::{ApiMode, ProviderTransport, StreamResult},
    types::{
        ContentPart, InferenceConfig, Message, StopReason, TokenUsage, ToolSchema,
        TransportResponse,
    },
};
use garudust_tools::ToolRegistry;

use crate::Agent;

// ── Minimal stubs ─────────────────────────────────────────────────────────────

struct StaticTransport {
    reply: String,
}

#[async_trait]
impl ProviderTransport for StaticTransport {
    fn api_mode(&self) -> ApiMode {
        ApiMode::ChatCompletions
    }

    async fn chat(
        &self,
        _messages: &[Message],
        _config: &InferenceConfig,
        _tools: &[ToolSchema],
    ) -> Result<TransportResponse, TransportError> {
        Ok(TransportResponse {
            content: vec![ContentPart::Text(self.reply.clone())],
            tool_calls: vec![],
            usage: TokenUsage::default(),
            stop_reason: StopReason::EndTurn,
        })
    }

    async fn chat_stream(
        &self,
        _messages: &[Message],
        _config: &InferenceConfig,
        _tools: &[ToolSchema],
    ) -> Result<StreamResult, TransportError> {
        use futures::stream;
        use garudust_core::types::StreamChunk;

        let chunks = vec![
            Ok(StreamChunk::TextDelta(self.reply.clone())),
            Ok(StreamChunk::Done {
                usage: TokenUsage::default(),
            }),
        ];
        Ok(Box::pin(stream::iter(chunks)))
    }
}

struct NopMemory;

#[async_trait]
impl MemoryStore for NopMemory {
    async fn read_memory(&self) -> Result<MemoryContent, AgentError> {
        Ok(MemoryContent::default())
    }
    async fn write_memory(&self, _: &MemoryContent) -> Result<(), AgentError> {
        Ok(())
    }
    async fn read_user_profile(&self) -> Result<String, AgentError> {
        Ok(String::new())
    }
    async fn write_user_profile(&self, _: &str) -> Result<(), AgentError> {
        Ok(())
    }
}

struct AutoApprove;
#[async_trait]
impl CommandApprover for AutoApprove {
    async fn approve(&self, _: &str, _: &str) -> ApprovalDecision {
        ApprovalDecision::Approved
    }
}

fn make_agent(reply: &str) -> Arc<Agent> {
    let config = Arc::new(AgentConfig::default());
    make_agent_with_config(reply, config)
}

fn make_agent_with_config(reply: &str, config: Arc<AgentConfig>) -> Arc<Agent> {
    let transport = Arc::new(StaticTransport {
        reply: reply.to_string(),
    });
    let tools = Arc::new(ToolRegistry::new());
    let memory = Arc::new(NopMemory);
    Arc::new(Agent::new(transport, tools, memory, config))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn spawn_child_has_independent_budget() {
    let config = AgentConfig {
        max_iterations: 5,
        ..AgentConfig::default()
    };
    let parent = make_agent_with_config("hi", Arc::new(config));

    parent.consume_budget(); // parent uses 1 → 4 remaining
    let child = parent.spawn_child();

    assert_eq!(child.budget_remaining(), 5, "child starts with full budget");
    assert_eq!(
        parent.budget_remaining(),
        4,
        "parent budget unaffected by child creation"
    );

    child.consume_budget(); // child uses 1 → 4 remaining
    assert_eq!(
        parent.budget_remaining(),
        4,
        "parent unaffected by child consumption"
    );
}

#[tokio::test]
async fn run_returns_reply() {
    let agent = make_agent("Hello, world!");
    let result = agent
        .run("say hi", Arc::new(AutoApprove), "test")
        .await
        .unwrap();
    assert_eq!(result.output, "Hello, world!");
    assert_eq!(result.iterations, 1);
}

#[tokio::test]
async fn run_streaming_emits_chunks() {
    let agent = make_agent("streamed response");
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let result = agent
        .run_streaming("say something", Arc::new(AutoApprove), "test", tx)
        .await
        .unwrap();

    // Collect all chunks
    let mut chunks = Vec::new();
    while let Ok(c) = rx.try_recv() {
        chunks.push(c);
    }
    assert_eq!(chunks.join(""), "streamed response");
    assert_eq!(result.output, "streamed response");
}
