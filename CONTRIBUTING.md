# Contributing to Garudust

Thanks for your interest in contributing to Garudust!

## Quick Start

```bash
# Clone the repo
git clone https://github.com/garudust-org/garudust-agent.git
cd garudust-agent

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
| `crates/garudust-transport` | LLM provider implementations — Anthropic, OpenAI-compatible (OpenRouter, etc.), AWS Bedrock, Codex, Ollama, vLLM |
| `crates/garudust-tools` | Built-in tools: `web_fetch`, `web_search`, `http_request`, `browser`, `read_file`, `write_file`, `list_directory`, `pdf_read`, `terminal`, `memory`, `user_profile`, `session_search`, `delegate_task`, `skills_list`, `skill_view`, `write_skill` |
| `crates/garudust-memory` | Persistence: `FileMemoryStore` (markdown files) + `SessionDb` (SQLite + FTS5) |
| `crates/garudust-agent` | Agent run loop, context compression, session persistence, `AutoApprover` / `ConstitutionalApprover` / `DenyApprover` |
| `crates/garudust-platforms` | Platform adapters: Telegram, Discord, Slack (Socket Mode), Matrix, LINE, Webhook |
| `crates/garudust-cron` | Cron scheduler — wraps `tokio-cron-scheduler`, spawns agent on schedule |
| `crates/garudust-gateway` | HTTP gateway — Bearer auth middleware, rate limiting, `GatewayHandler`, `/health` + `/chat*` routes |
| `bin/garudust` | CLI binary: TUI chat, `setup`, `config show/set`, `doctor` |
| `bin/garudust-server` | Headless server: all platform adapters + HTTP API + Cron in one process |

Each crate has a single focused responsibility. Keep those boundaries clean.

## Finding Work

- Check the [Issues](https://github.com/garudust-org/garudust-agent/issues) page
- Issues labeled `good first issue` are great starting points
- Comment on an issue before starting work to avoid duplicate effort

## Branch & Pull Request Workflow

**All changes go through a branch and a PR — no direct pushes to `main`, including from maintainers.**

### Branch naming

| Prefix | Use for |
|--------|---------|
| `feat/` | New feature or tool |
| `fix/` | Bug fix or clippy/fmt correction |
| `refactor/` | Code restructuring without behaviour change |
| `docs/` | Documentation only |
| `chore/` | Dependency bumps, CI config, repo hygiene |
| `perf/` | Performance improvement |

```bash
# Create and switch to a new branch from main
git checkout main && git pull
git checkout -b feat/my-feature
```

### Checklist before opening a PR

```bash
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -W clippy::all -W clippy::pedantic
cargo fmt --all -- --check
cargo test --workspace
```

All four must be green. CI runs the same commands with `RUSTFLAGS=-D warnings`.

If you changed `Cargo.toml` (added/removed/bumped a dependency), stage `Cargo.lock` too:

```bash
git add Cargo.lock
```

`Cargo.lock` is committed in this repo because it pins exact dependency versions for reproducible builds. Leaving it out of the PR causes a stale-lock diff on `main`.

### Opening the PR

- Title: one Conventional Commit line (`feat(tools): add http_request tool`)
- Body: what changed, why, and how to test it
- Link the issue it closes (`Closes #67`)
- CI must be green before merge

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
use serde::Deserialize;
use serde_json::{json, Value};

// Use a typed struct for params — avoids manual `.as_str()` / `.ok_or_else()` boilerplate
// and gives you free validation before execute() is called.
#[derive(Deserialize)]
struct YourToolInput {
    input: String,
    optional_flag: Option<bool>,
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
                "input": { "type": "string", "description": "The input" },
                "optional_flag": { "type": "boolean" }
            },
            "required": ["input"]
        })
    }

    // Override only if the tool manages its own timeout internally (e.g. it
    // accepts a user-supplied timeout_secs param). The default returns false,
    // which means the global tool_timeout_secs from config.yaml applies.
    fn bypass_dispatch_timeout(&self) -> bool { false }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let input: YourToolInput = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidArgs(e.to_string()))?;
        Ok(ToolResult::ok("", format!("Processed: {}", input.input)))
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

pub struct YourAdapter { /* token, http client, etc. */ }

#[async_trait]
impl PlatformAdapter for YourAdapter {
    fn name(&self) -> &'static str { "your_platform" }

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
