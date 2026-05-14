# Contributing to Garudust

No contribution is too small — bug reports, docs fixes, new tools, or new platform adapters are all welcome.

## Quick Start

```bash
git clone https://github.com/garudust-org/garudust-agent.git
cd garudust-agent
cargo build
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --all
```

## Crate Overview

| Crate / Binary | Purpose |
|----------------|---------|
| `crates/garudust-core` | Shared types and traits (`Tool`, `ProviderTransport`, `PlatformAdapter`, `MemoryStore`), config, security |
| `crates/garudust-transport` | LLM provider implementations — Anthropic, OpenAI-compat, Ollama, vLLM, Bedrock |
| `crates/garudust-tools` | Built-in tools: `web_fetch`, `web_search`, `browser`, `read_file`, `write_file`, `terminal`, `memory`, and more |
| `crates/garudust-memory` | Persistence: `FileMemoryStore` (markdown) + `SessionDb` (SQLite + FTS5) |
| `crates/garudust-agent` | Agent run loop, context compression, session persistence, approvers |
| `crates/garudust-platforms` | Platform adapters: Telegram, Discord, Slack, Matrix, LINE, WhatsApp, Webhook |
| `crates/garudust-cron` | Cron scheduler — wraps `tokio-cron-scheduler` |
| `crates/garudust-gateway` | HTTP gateway — auth, rate limiting, `/health` + `/chat*` routes |
| `bin/garudust` | CLI: TUI chat, `setup`, `config`, `doctor` |
| `bin/garudust-server` | Headless server: all platform adapters + HTTP API + cron |

## Finding Work

- Browse [Issues](https://github.com/garudust-org/garudust-agent/issues) — look for `good first issue`
- Comment on an issue before you start to avoid duplicate effort
- Not sure where to begin? Adding a new tool is the easiest entry point

## Before Opening a PR

Run these locally — CI checks the same things:

```bash
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets
cargo fmt --all -- --check
cargo test --workspace
```

Stage `Cargo.lock` if you changed `Cargo.toml`:

```bash
git add Cargo.lock
```

PR title should follow [Conventional Commits](https://www.conventionalcommits.org/):
`feat(tools): add image generation tool` · `fix: handle empty tool_calls` · `docs: update readme`

## How to Add a New Tool

Create `crates/garudust-tools/src/toolsets/your_tool.rs`:

```rust
use async_trait::async_trait;
use garudust_core::{error::ToolError, tool::{Tool, ToolContext}, types::ToolResult};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
struct YourToolInput {
    input: String,
}

pub struct YourTool;

#[async_trait]
impl Tool for YourTool {
    fn name(&self) -> &'static str { "your_tool" }
    fn description(&self) -> &'static str { "Does something useful" }
    fn toolset(&self) -> &'static str { "your_toolset" }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "input": { "type": "string", "description": "The input" }
            },
            "required": ["input"]
        })
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let input: YourToolInput = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidArgs(e.to_string()))?;
        Ok(ToolResult::ok("", format!("Processed: {}", input.input)))
    }
}
```

Register it in `bin/garudust/src/main.rs` and `bin/garudust-server/src/main.rs`:

```rust
registry.register(YourTool);
```

## How to Add a New Platform Adapter

Implement `PlatformAdapter` from `garudust-core` in `crates/garudust-platforms/`:

```rust
use async_trait::async_trait;
use garudust_core::{error::PlatformError, platform::{MessageHandler, PlatformAdapter}, types::{ChannelId, OutboundMessage}};

pub struct YourAdapter { /* token, http client, etc. */ }

#[async_trait]
impl PlatformAdapter for YourAdapter {
    fn name(&self) -> &'static str { "your_platform" }

    async fn start(&self, handler: Arc<dyn MessageHandler>) -> Result<(), PlatformError> {
        // Spawn listener, call handler.handle(inbound) on each message
        Ok(())
    }

    async fn send_message(&self, channel: &ChannelId, message: OutboundMessage) -> Result<(), PlatformError> {
        Ok(())
    }

    async fn send_stream(&self, channel: &ChannelId, mut stream: Pin<Box<dyn Stream<Item = String> + Send>>) -> Result<(), PlatformError> {
        Ok(())
    }
}
```

## How to Add a New LLM Transport

Implement `ProviderTransport` from `garudust-core` and register it in `crates/garudust-transport/src/registry.rs`:

```rust
use async_trait::async_trait;
use garudust_core::{error::TransportError, transport::ProviderTransport, types::{InferenceConfig, InferenceResponse, Message, ToolSchema}};

pub struct YourTransport;

#[async_trait]
impl ProviderTransport for YourTransport {
    async fn chat(
        &self,
        messages: &[Message],
        config: &InferenceConfig,
        tools: &[ToolSchema],
    ) -> Result<InferenceResponse, TransportError> {
        todo!()
    }
}
```

## Security

For vulnerabilities, please see [SECURITY.md](SECURITY.md) and report privately rather than opening a public issue.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
