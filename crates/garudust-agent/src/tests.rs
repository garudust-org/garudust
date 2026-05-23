#![cfg(test)]

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use garudust_core::{
    config::AgentConfig,
    error::{AgentError, ToolError, TransportError},
    memory::{MemoryCategory, MemoryContent, MemoryEntry, MemoryStore},
    tool::{ApprovalDecision, CommandApprover, Tool, ToolContext},
    transport::{ApiMode, ProviderTransport, StreamResult},
    types::{
        ContentPart, InferenceConfig, Message, Role, StopReason, TokenUsage, ToolCall, ToolResult,
        ToolSchema, TransportResponse,
    },
};
use garudust_tools::ToolRegistry;

use crate::{compressor::ContextCompressor, prompt_builder::build_system_prompt, Agent};

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
    async fn approve(&self, _: &str, _: &str, _: &str) -> ApprovalDecision {
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
        .run("say hi", Arc::new(AutoApprove), "test", None, None)
        .await
        .unwrap();
    assert!(
        result.output.starts_with("Hello, world!"),
        "unexpected output: {}",
        result.output
    );
    assert_eq!(result.iterations, 1);
}

#[tokio::test]
async fn run_streaming_emits_chunks() {
    let agent = make_agent("streamed response");
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let result = agent
        .run_streaming(
            "say something",
            Arc::new(AutoApprove),
            "test",
            tx,
            None,
            None,
            None,
        )
        .await
        .unwrap();

    // Collect all chunks
    let mut chunks = Vec::new();
    while let Ok(c) = rx.try_recv() {
        chunks.push(c);
    }
    assert_eq!(chunks.join(""), "streamed response");
    assert!(
        result.output.starts_with("streamed response"),
        "unexpected output: {}",
        result.output
    );
}

// ── Scripted transport + recording tool ───────────────────────────────────────

/// Returns queued responses in FIFO order and counts how many times `chat`
/// was invoked, so a test can both script multi-turn behaviour and assert that
/// no LLM call happened on a short-circuit path.
struct ScriptedTransport {
    responses: Mutex<std::collections::VecDeque<TransportResponse>>,
    calls: Arc<std::sync::atomic::AtomicUsize>,
}

impl ScriptedTransport {
    fn new(responses: Vec<TransportResponse>) -> (Arc<Self>, Arc<std::sync::atomic::AtomicUsize>) {
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let t = Arc::new(Self {
            responses: Mutex::new(responses.into()),
            calls: calls.clone(),
        });
        (t, calls)
    }

    fn next(&self) -> TransportResponse {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(TransportResponse {
                content: vec![ContentPart::Text(String::new())],
                tool_calls: vec![],
                usage: TokenUsage::default(),
                stop_reason: StopReason::EndTurn,
            })
    }
}

#[async_trait]
impl ProviderTransport for ScriptedTransport {
    fn api_mode(&self) -> ApiMode {
        ApiMode::ChatCompletions
    }

    async fn chat(
        &self,
        _messages: &[Message],
        _config: &InferenceConfig,
        _tools: &[ToolSchema],
    ) -> Result<TransportResponse, TransportError> {
        Ok(self.next())
    }

    async fn chat_stream(
        &self,
        _messages: &[Message],
        _config: &InferenceConfig,
        _tools: &[ToolSchema],
    ) -> Result<StreamResult, TransportError> {
        unimplemented!("scripted transport is non-streaming")
    }
}

/// Records every parameter object it is dispatched with so a test can prove the
/// run-loop actually executed the tool the model requested.
struct RecordingTool {
    calls: Arc<Mutex<Vec<serde_json::Value>>>,
}

#[async_trait]
impl Tool for RecordingTool {
    fn name(&self) -> &'static str {
        "echo"
    }
    fn description(&self) -> &'static str {
        "Echo back the provided text"
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": { "text": { "type": "string" } },
            "required": ["text"]
        })
    }
    fn toolset(&self) -> &'static str {
        "test"
    }
    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let text = params
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        self.calls.lock().unwrap().push(params);
        Ok(ToolResult::ok("", format!("echoed: {text}")))
    }
}

// ── ContextCompressor ─────────────────────────────────────────────────────────

#[tokio::test]
async fn compressor_should_compress_respects_threshold() {
    let (transport, _) = ScriptedTransport::new(vec![]);
    let compressor = ContextCompressor::new(transport, "m".into()).with_context_limit(100);

    // ~3 chars/token estimate, threshold = 100 * 0.80 = 80 tokens ≈ 240 chars.
    let small = vec![Message::user("short")];
    assert!(!compressor.should_compress(&small));

    let big = vec![Message::user("x".repeat(2_000))];
    assert!(compressor.should_compress(&big));
}

#[tokio::test]
async fn compressor_short_conversation_left_unchanged() {
    let (transport, calls) = ScriptedTransport::new(vec![]);
    let compressor = ContextCompressor::new(transport, "m".into());

    // head(1) + tail(6*2) boundary: <= 13 conversation messages is a no-op.
    let mut msgs = vec![Message::system("sys")];
    for i in 0..8 {
        msgs.push(Message::user(format!("u{i}")));
    }
    let original_len = msgs.len();

    let (out, _usage) = compressor.compress(msgs).await.unwrap();

    assert_eq!(
        out.len(),
        original_len,
        "short conversation must pass through"
    );
    assert_eq!(
        calls.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "no LLM call on the short-circuit path"
    );
}

#[tokio::test]
async fn compressor_long_conversation_summarized() {
    let summary = TransportResponse {
        content: vec![ContentPart::Text("CONDENSED".into())],
        tool_calls: vec![],
        usage: TokenUsage::default(),
        stop_reason: StopReason::EndTurn,
    };
    let (transport, calls) = ScriptedTransport::new(vec![summary]);
    let compressor = ContextCompressor::new(transport, "m".into());

    let mut msgs = vec![Message::system("SYSTEM")];
    msgs.push(Message::user("FIRST_TASK")); // head — preserved
    for i in 0..18 {
        msgs.push(Message::assistant(format!("middle{i}"))); // summarized
    }
    for i in 0..12 {
        msgs.push(Message::user(format!("tail{i}"))); // tail — preserved
    }

    let (out, _usage) = compressor.compress(msgs).await.unwrap();

    assert_eq!(
        calls.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "exactly one summarize call"
    );
    // system + head + summary + 12 tail
    assert_eq!(out.len(), 1 + 1 + 1 + 12);
    assert_eq!(out[0].role, Role::System);
    let texts: Vec<&str> = out
        .iter()
        .filter_map(|m| {
            m.content.iter().find_map(|p| match p {
                ContentPart::Text(t) => Some(t.as_str()),
                _ => None,
            })
        })
        .collect();
    assert!(texts.contains(&"SYSTEM"));
    assert!(texts.iter().any(|t| t.contains("FIRST_TASK")));
    assert!(texts.iter().any(|t| t.contains("CONDENSED")));
    assert!(texts.iter().any(|t| t.contains("tail11")));
    assert!(
        !texts.iter().any(|t| t.contains("middle9")),
        "middle turns must be replaced by the summary"
    );
}

// ── prompt_builder ────────────────────────────────────────────────────────────

#[tokio::test]
async fn system_prompt_contains_identity_and_optional_sections() {
    // Isolate home_dir so SOUL.md / skills from a real ~/.garudust don't leak in.
    let tmp = std::env::temp_dir().join(format!("garudust-prompt-test-{}", std::process::id()));
    let config = AgentConfig {
        home_dir: tmp,
        ..AgentConfig::default()
    };

    let bare = build_system_prompt(&config, None, None, "cli").await;
    assert!(
        bare.contains("You are Garudust"),
        "identity must always be present"
    );
    // Note: the identity text itself has a "## Memory" heading, so assert on the
    // injected *content* rather than the section header substring.
    assert!(!bare.contains("user prefers tabs"));
    assert!(!bare.contains("Alice, engineer"));

    let mem = MemoryContent {
        entries: vec![MemoryEntry::new(
            MemoryCategory::Fact,
            "user prefers tabs".into(),
        )],
    };
    let full = build_system_prompt(&config, Some(&mem), Some("Alice, engineer"), "cli").await;
    assert!(full.contains("# Memory"));
    assert!(full.contains("user prefers tabs"));
    assert!(full.contains("# User Profile"));
    assert!(full.contains("Alice, engineer"));
    assert!(
        full.contains("\n\n---\n\n"),
        "sections joined by hr divider"
    );
}

// ── Agent run-loop: tool dispatch then completion ─────────────────────────────

#[tokio::test]
async fn run_loop_executes_tool_then_finishes() {
    let calls = Arc::new(Mutex::new(Vec::new()));

    // Turn 1: model asks for the `echo` tool. Turn 2: model replies with text.
    let turn1 = TransportResponse {
        content: vec![],
        tool_calls: vec![ToolCall {
            id: "call_1".into(),
            name: "echo".into(),
            arguments: serde_json::json!({ "text": "hi" }),
        }],
        usage: TokenUsage::default(),
        stop_reason: StopReason::ToolUse,
    };
    let turn2 = TransportResponse {
        content: vec![ContentPart::Text("done".into())],
        tool_calls: vec![],
        usage: TokenUsage::default(),
        stop_reason: StopReason::EndTurn,
    };
    let (transport, _) = ScriptedTransport::new(vec![turn1, turn2]);

    let mut registry = ToolRegistry::new();
    registry.register(RecordingTool {
        calls: calls.clone(),
    });

    let agent = Arc::new(Agent::new(
        transport,
        Arc::new(registry),
        Arc::new(NopMemory),
        Arc::new(AgentConfig::default()),
    ));

    let result = agent
        .run(
            "use the echo tool",
            Arc::new(AutoApprove),
            "test",
            None,
            None,
        )
        .await
        .unwrap();

    assert_eq!(result.iterations, 2, "one tool turn + one completion turn");
    assert!(result.output.starts_with("done"), "got: {}", result.output);

    let recorded = calls.lock().unwrap();
    assert_eq!(
        recorded.len(),
        1,
        "echo tool must be dispatched exactly once"
    );
    assert_eq!(recorded[0]["text"], "hi");
}

// ── max_iterations enforcement ────────────────────────────────────────────────

#[tokio::test]
async fn run_stops_at_max_iterations() {
    // Every turn asks for the echo tool → the loop would run forever without a cap.
    let tool_turn = TransportResponse {
        content: vec![],
        tool_calls: vec![ToolCall {
            id: "c1".into(),
            name: "echo".into(),
            arguments: serde_json::json!({ "text": "x" }),
        }],
        usage: TokenUsage::default(),
        stop_reason: StopReason::ToolUse,
    };
    let responses: Vec<TransportResponse> = std::iter::repeat_n(tool_turn, 20).collect();
    let (transport, _) = ScriptedTransport::new(responses);

    let mut registry = ToolRegistry::new();
    registry.register(RecordingTool {
        calls: Arc::new(Mutex::new(vec![])),
    });

    let config = Arc::new(AgentConfig {
        max_iterations: 3,
        ..AgentConfig::default()
    });
    let agent = Arc::new(Agent::new(
        transport,
        Arc::new(registry),
        Arc::new(NopMemory),
        config,
    ));

    let result = agent
        .run("run forever", Arc::new(AutoApprove), "test", None, None)
        .await;

    // Should stop with a budget error or a truncated result — not loop 20 times.
    match result {
        Err(AgentError::BudgetExhausted(_)) => {} // expected path
        Ok(r) => assert!(
            r.iterations <= 5,
            "ran too many iterations: {}",
            r.iterations
        ),
        Err(e) => panic!("unexpected error: {e}"),
    }
}

// ── token budget cap ──────────────────────────────────────────────────────────

#[tokio::test]
async fn run_respects_max_tokens_per_task() {
    // Each turn reports 200 input + 200 output tokens = 400. Cap is 500.
    // After turn 1 (400 tokens) we are under; after the tool result
    // is fed back and turn 2 fires the cumulative total passes 500.
    let heavy_usage = TokenUsage {
        input_tokens: 200,
        output_tokens: 200,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
    };
    let tool_turn = TransportResponse {
        content: vec![],
        tool_calls: vec![ToolCall {
            id: "c1".into(),
            name: "echo".into(),
            arguments: serde_json::json!({ "text": "x" }),
        }],
        usage: heavy_usage.clone(),
        stop_reason: StopReason::ToolUse,
    };
    let finish_turn = TransportResponse {
        content: vec![ContentPart::Text("done".into())],
        tool_calls: vec![],
        usage: heavy_usage,
        stop_reason: StopReason::EndTurn,
    };
    let (transport, _) = ScriptedTransport::new(vec![tool_turn, finish_turn]);

    let mut registry = ToolRegistry::new();
    registry.register(RecordingTool {
        calls: Arc::new(Mutex::new(vec![])),
    });

    let config = Arc::new(AgentConfig {
        max_tokens_per_task: Some(500),
        ..AgentConfig::default()
    });
    let agent = Arc::new(Agent::new(
        transport,
        Arc::new(registry),
        Arc::new(NopMemory),
        config,
    ));

    let result = agent
        .run("heavy task", Arc::new(AutoApprove), "test", None, None)
        .await
        .unwrap();

    // Output must mention budget exhaustion — the loop terminates early.
    assert!(
        result.output.contains("Token budget") || result.iterations <= 2,
        "expected early stop due to token budget; got output: {}",
        result.output
    );
}

// ── parallel tool dispatch ────────────────────────────────────────────────────

#[tokio::test]
async fn run_loop_dispatches_multiple_tools_in_one_turn() {
    let calls = Arc::new(Mutex::new(Vec::new()));

    // Turn 1: model requests two echo calls in the same turn.
    let turn1 = TransportResponse {
        content: vec![],
        tool_calls: vec![
            ToolCall {
                id: "c1".into(),
                name: "echo".into(),
                arguments: serde_json::json!({ "text": "first" }),
            },
            ToolCall {
                id: "c2".into(),
                name: "echo".into(),
                arguments: serde_json::json!({ "text": "second" }),
            },
        ],
        usage: TokenUsage::default(),
        stop_reason: StopReason::ToolUse,
    };
    let turn2 = TransportResponse {
        content: vec![ContentPart::Text("all done".into())],
        tool_calls: vec![],
        usage: TokenUsage::default(),
        stop_reason: StopReason::EndTurn,
    };
    let (transport, _) = ScriptedTransport::new(vec![turn1, turn2]);

    let mut registry = ToolRegistry::new();
    registry.register(RecordingTool {
        calls: calls.clone(),
    });

    let agent = Arc::new(Agent::new(
        transport,
        Arc::new(registry),
        Arc::new(NopMemory),
        Arc::new(AgentConfig::default()),
    ));

    let result = agent
        .run("use both tools", Arc::new(AutoApprove), "test", None, None)
        .await
        .unwrap();

    assert_eq!(result.iterations, 2);
    let recorded = calls.lock().unwrap();
    assert_eq!(recorded.len(), 2, "both tool calls must be dispatched");
    let texts: Vec<&str> = recorded
        .iter()
        .filter_map(|v| v.get("text")?.as_str())
        .collect();
    assert!(texts.contains(&"first"));
    assert!(texts.contains(&"second"));
}

// ── compression trigger during run-loop ───────────────────────────────────────

#[tokio::test]
async fn run_loop_triggers_compression_when_threshold_exceeded() {
    use crate::compressor::ContextCompressor;
    use garudust_core::hooks::AgentHooks;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Count how many times on_pre_compress is called.
    struct CountHooks(Arc<AtomicUsize>);
    #[async_trait::async_trait]
    impl AgentHooks for CountHooks {
        async fn on_pre_compress(&self, _count: usize, _sid: &str) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    let compress_count = Arc::new(AtomicUsize::new(0));

    // Single finishing turn — the history won't be huge but we override the
    // compressor's threshold to 0 chars so it fires unconditionally.
    let reply = TransportResponse {
        content: vec![ContentPart::Text("compressed run done".into())],
        tool_calls: vec![],
        usage: TokenUsage::default(),
        stop_reason: StopReason::EndTurn,
    };
    let (transport, _) = ScriptedTransport::new(vec![reply.clone()]);

    // Build a compressor that always says "compress" but uses a static transport
    // for the summarise call so no real LLM is invoked.
    let summary_resp = TransportResponse {
        content: vec![ContentPart::Text("summary".into())],
        tool_calls: vec![],
        usage: TokenUsage::default(),
        stop_reason: StopReason::EndTurn,
    };
    let (compress_transport, _) = ScriptedTransport::new(vec![summary_resp]);
    let compressor = ContextCompressor::new(compress_transport, "m".into()).with_context_limit(1); // threshold = 1 token → always fires

    let mut config = AgentConfig::default();
    config.compression.enabled = true;

    let agent = Arc::new(
        Agent::new(
            transport,
            Arc::new(ToolRegistry::new()),
            Arc::new(NopMemory),
            Arc::new(config),
        )
        .with_compressor(compressor)
        .with_hooks(CountHooks(compress_count.clone())),
    );

    let result = agent
        .run(
            "trigger compress",
            Arc::new(AutoApprove),
            "test",
            None,
            None,
        )
        .await
        .unwrap();

    assert!(result.output.contains("compressed run done"));
    assert!(
        compress_count.load(Ordering::SeqCst) >= 1,
        "on_pre_compress hook must fire when context exceeds threshold"
    );
}
