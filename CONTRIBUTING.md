# Contributing to Garudust

Thanks for your interest in contributing to Garudust!

## Quick Start

```bash
# Clone the repo
git clone https://github.com/ninenox/garudust.git
cd garudust

# Build everything
cargo build

# Check for errors
cargo check --workspace --all-targets

# Run linter
cargo clippy --workspace --all-targets

# Format code (required before PR)
cargo fmt --all

# Verify formatting (what CI checks)
cargo fmt --all -- --check
```

## Crate Overview

| Crate / Binary | Purpose |
|----------------|---------|
| `crates/garudust-core` | Shared types, traits (`Tool`, `ProviderTransport`, `PlatformAdapter`, `MemoryStore`), config, `SecurityConfig`, `net_guard` (SSRF) |
| `crates/garudust-transport` | LLM provider implementations — Anthropic, OpenAI-compatible (OpenRouter, etc.), AWS Bedrock, Codex |
| `crates/garudust-tools` | Built-in tools: `read_file`, `write_file`, `terminal`, `web_fetch`, `web_search`, `browser`, `memory`, `delegate_task`, `skills_list`, `skill_view` |
| `crates/garudust-memory` | Persistence: `FileMemoryStore` (markdown files) + `SessionDb` (SQLite + FTS5) |
| `crates/garudust-agent` | Agent run loop, context compression, session persistence, `AutoApprover` / `SmartApprover` / `DenyApprover` |
| `crates/garudust-platforms` | Platform adapters: Telegram, Discord, Slack (Socket Mode), Matrix, Webhook |
| `crates/garudust-cron` | Cron scheduler — wraps `tokio-cron-scheduler`, spawns agent on schedule |
| `crates/garudust-gateway` | HTTP gateway — Bearer auth middleware, rate limiting, `GatewayHandler`, `/health` + `/chat*` routes |
| `bin/garudust` | CLI binary: TUI chat, `setup`, `config show/set`, `doctor` |
| `bin/garudust-server` | Headless server: all platform adapters + HTTP API + Cron in one process |

Each crate has a single focused responsibility. Keep those boundaries clean.

## Finding Work

- Check the [Issues](https://github.com/ninenox/garudust/issues) page
- Issues labeled `good first issue` are great starting points
- Comment on an issue before starting work to avoid duplicate effort

## Pull Request Process

1. Fork the repository and create a feature branch from `main`
   ```bash
   git checkout -b feat/my-feature
   ```
2. Make your changes
3. Ensure all checks pass:
   ```bash
   cargo check --workspace --all-targets
   cargo clippy --workspace --all-targets
   cargo fmt --all -- --check
   ```
4. Submit a PR with a clear description of what changed and why

## Code Guidelines

- **One concern per crate.** Keep dependency direction contract-first: concrete integrations depend on shared traits in `garudust-core`, not on each other.
- **Traits before structs.** Define the trait in `garudust-core`, implement it elsewhere.
- **`Result<T, E>` over panics.** Use `?`, `anyhow`, or `thiserror` — no `.unwrap()` in production paths.
- **No comments that describe what the code does.** Only comment the *why* when it is non-obvious.
- **Minimal dependencies.** Every added crate increases compile time and binary size. Prefer the standard library or existing workspace deps.

## Naming Conventions

- Modules / files → `snake_case`
- Types / traits / enums → `PascalCase`
- Functions / variables → `snake_case`
- Constants → `SCREAMING_SNAKE_CASE`
- Prefer domain-first names: `TelegramAdapter`, `WebSearch`, `SessionDb` — not `Manager`, `Helper`, `Util`.
- Trait implementers use a consistent suffix: `*Adapter` (platform), `*Transport` (LLM), `*Store` (memory).

## How to Add a New Tool

Create `crates/garudust-tools/src/toolsets/your_tool.rs`:

```rust
use async_trait::async_trait;
use garudust_core::{error::ToolError, tool::{Tool, ToolContext}, types::ToolResult};
use serde_json::{json, Value};

pub struct YourTool;

#[async_trait]
impl Tool for YourTool {
    fn name(&self) -> &str { "your_tool" }
    fn description(&self) -> &str { "Does something useful" }
    fn toolset(&self) -> &str { "your_toolset" }

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
        let input = params["input"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("'input' required".into()))?;
        Ok(ToolResult::ok("", format!("Processed: {input}")))
    }
}
```

Then register it in `bin/garudust/src/main.rs` and `bin/garudust-server/src/main.rs`:

```rust
registry.register(YourTool);
```

## How to Add a New Platform Adapter

Implement `PlatformAdapter` from `garudust-core`:

```rust
use async_trait::async_trait;
use garudust_core::{error::PlatformError, platform::{MessageHandler, PlatformAdapter}, types::{ChannelId, OutboundMessage}};

pub struct SlackAdapter { /* token, http client, etc. */ }

#[async_trait]
impl PlatformAdapter for SlackAdapter {
    fn name(&self) -> &'static str { "slack" }

    async fn start(&self, handler: Arc<dyn MessageHandler>) -> Result<(), PlatformError> {
        // Spawn listener, call handler.handle(inbound) on each message
        Ok(())
    }

    async fn send_message(&self, channel: &ChannelId, message: OutboundMessage) -> Result<(), PlatformError> {
        // POST to Slack API
        Ok(())
    }

    async fn send_stream(&self, channel: &ChannelId, mut stream: Pin<Box<dyn Stream<Item = String> + Send>>) -> Result<(), PlatformError> {
        // Buffer stream and call send_message, or implement live typing
        Ok(())
    }
}
```

Add it to `crates/garudust-platforms/` and register behind a feature flag in `Cargo.toml`.

## How to Add a New LLM Transport

Implement `ProviderTransport` from `garudust-core`:

```rust
use async_trait::async_trait;
use garudust_core::{error::TransportError, transport::ProviderTransport, types::{InferenceConfig, InferenceResponse, Message, ToolSchema}};

pub struct YourTransport { /* client */ }

#[async_trait]
impl ProviderTransport for YourTransport {
    async fn chat(
        &self,
        messages: &[Message],
        config: &InferenceConfig,
        tools: &[ToolSchema],
    ) -> Result<InferenceResponse, TransportError> {
        // Call your provider API
        todo!()
    }
}
```

Register it in `crates/garudust-transport/src/registry.rs`.

## Commit Convention

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add Slack platform adapter
feat(tools): add image generation tool
fix: handle empty tool_calls in anthropic transport
docs: update contributing guide
refactor(agent): extract persist_session helper
chore: bump tokio to 1.44
ci: add matrix build for stable + beta
```

Recommended scope keys: `agent`, `tools`, `transport`, `memory`, `platforms`, `gateway`, `cron`, `cli`, `ci`, `docs`.

## Secret Hygiene

Before every commit, verify:

- No `.env` files are staged (`git status` should not show `.env`)
- No raw API keys or tokens in code, tests, or fixtures
- `git diff --cached | grep -iE '(api[_-]?key|secret|token|bearer|sk-)'` returns nothing

`~/.garudust/.env` and `~/.garudust/config.yaml` are user-local and git-ignored by default.

## Security

Security-sensitive changes (new tool with network access, auth logic, file I/O) should be accompanied by a note in the PR explaining the threat model. See [SECURITY.md](SECURITY.md) for the project's security policy and how to report vulnerabilities privately.

## CI

Every PR runs:

| Job | Command |
|-----|---------|
| Check & Clippy | `cargo check --workspace --all-targets` + `cargo clippy` (pedantic) |
| Rustfmt | `cargo fmt --all -- --check` |

All jobs must be green before merge. `RUSTFLAGS=-D warnings` is set — warnings are errors in CI.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
