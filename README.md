<div align="center">
  <img src="assets/logo.svg" alt="Garudust" width="260"/>
</div>

# Garudust

**A self-hostable AI agent runtime written in Rust.**

Talk to it from your terminal, wire it into Telegram / Discord, or call it from your own app over HTTP — all from a single binary.

[![CI](https://github.com/ninenox/garudust/actions/workflows/ci.yml/badge.svg)](https://github.com/ninenox/garudust/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Rust 1.75+](https://img.shields.io/badge/rust-1.75+-orange.svg)

---

## Why Garudust?

Most AI agent frameworks are Python, heavy, and slow to start. Garudust is:

- **~10 MB binary, < 20 ms cold start** — no Python runtime, no Docker required for local use
- **Swap providers with one env var** — Anthropic, OpenRouter, AWS Bedrock, OpenAI Responses API, or any OpenAI-compatible endpoint
- **Runs everywhere** — laptop TUI, headless server, Docker, Telegram bot, Discord bot, HTTP API
- **Composable** — every piece is a separate crate with a clean trait boundary; add a tool, platform, or transport without touching anything else

---

## Demo

```
$ garudust "find all TODO comments in this repo and open a GitHub issue for each one"

  AI › Scanning repository for TODO comments…
       Found 4 TODOs across 3 files.
       Creating issue #12: "Implement rate limiting in gateway"…
       Creating issue #13: "Add browser tool via CDP"…
       Done — 2 issues opened.
```

---

## Quick Start

### Option A — Docker (fastest)

```bash
# Copy env vars and start
echo "OPENROUTER_API_KEY=sk-or-..." > .env
docker compose up
```

Then call it:

```bash
curl -X POST http://localhost:3000/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "what is 2+2?"}'
```

### Option B — Build from source

```bash
git clone https://github.com/ninenox/garudust
cd garudust
cargo build --release
export PATH="$PATH:$(pwd)/target/release"

garudust setup   # first-time wizard: picks provider, saves API key
garudust         # launch interactive TUI
```

**Prerequisites:** Rust 1.75+ · An API key from [OpenRouter](https://openrouter.ai) or [Anthropic](https://console.anthropic.com)

---

## Usage

### Interactive TUI

```bash
garudust
```

| Key | Action |
|-----|--------|
| `Enter` | Send message |
| `↑ ↓` | Scroll history |
| `/new` | Clear history, start fresh session |
| `/model <name>` | Switch model on the fly |
| `/help` | Show all slash commands |
| `Ctrl+C` | Quit |

### One-shot task

```bash
garudust "summarise the git log from the last 7 days into a changelog"
garudust --model anthropic/claude-opus-4-7 "review this PR for security issues"
```

### Subcommands

| Command | Description |
|---------|-------------|
| `garudust setup` | First-time wizard |
| `garudust doctor` | Checks API key, connectivity, memory dir, session DB |
| `garudust config show` | Print active config |
| `garudust config set KEY VAL` | Set config value or secret |

```bash
garudust config set model                anthropic/claude-opus-4-7
garudust config set OPENROUTER_API_KEY   sk-or-...
garudust config set ANTHROPIC_API_KEY    sk-ant-...
```

---

## Headless Server

`garudust-server` starts everything at once: HTTP gateway, platform adapters, and cron jobs.

```bash
garudust-server \
  --anthropic-key  sk-ant-...   \
  --telegram-token 123456:ABC...\
  --discord-token  Bot_...
```

### HTTP API

```bash
# Blocking — waits for full response
curl -X POST http://localhost:3000/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "write a haiku about Rust"}'

# Streaming — Server-Sent Events, chunks arrive as the model writes
curl -X POST http://localhost:3000/chat/stream \
  -H "Content-Type: application/json" \
  -d '{"message": "explain async/await in 3 sentences"}'

# WebSocket — persistent connection, send JSON receive text chunks
# Connect: ws://localhost:3000/chat/ws
# Send:    {"message": "your task"}
# Receive: text chunks … then {"done":true}
```

### Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `OPENROUTER_API_KEY` | — | OpenRouter / any OpenAI-compatible key |
| `ANTHROPIC_API_KEY` | — | Anthropic key (auto-selects Anthropic transport) |
| `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` | — | AWS credentials for Bedrock transport |
| `BRAVE_SEARCH_API_KEY` | — | Enables `web_search` tool |
| `GARUDUST_MODEL` | `anthropic/claude-sonnet-4-6` | Model identifier |
| `GARUDUST_PORT` | `3000` | HTTP gateway port |
| `GARUDUST_WEBHOOK_PORT` | `3001` | Webhook adapter port (`0` = disabled) |
| `TELEGRAM_TOKEN` | — | Telegram bot token |
| `DISCORD_TOKEN` | — | Discord bot token |
| `GARUDUST_CRON_JOBS` | — | Comma-separated `"cron_expr=task"` pairs |
| `RUST_LOG` | `info` | Log level (`debug` for verbose) |

### Cron jobs

```bash
GARUDUST_CRON_JOBS="0 9 * * *=Write a morning briefing and save to ~/briefing.md" \
garudust-server
```

### Webhook

```bash
# Garudust receives POST /webhook, runs the agent, POSTs the reply to callback_url
curl -X POST http://localhost:3001/webhook \
  -H "Content-Type: application/json" \
  -d '{"text":"summarise today","callback_url":"https://your-app/reply"}'
```

---

## LLM Providers

| Provider | `provider` value | Notes |
|----------|-----------------|-------|
| Anthropic | `anthropic` | Direct Messages API |
| OpenRouter | `openrouter` *(default)* | Access 200+ models |
| OpenAI Responses API | `codex` | `/v1/responses` endpoint |
| AWS Bedrock | `bedrock` | Converse API, SigV4 auth from env |
| Any OpenAI-compatible | `openrouter` | Set `GARUDUST_BASE_URL` |

Switch provider:

```bash
garudust config set provider  bedrock
garudust config set model     anthropic.claude-3-5-sonnet-20241022-v2:0
```

---

## Built-in Tools

| Tool | Description |
|------|-------------|
| `web_fetch` | Fetch content from a URL |
| `web_search` | Web search via Brave Search |
| `browser` | Control Chrome/Chromium via CDP — navigate, click, type, screenshot, evaluate JS |
| `read_file` | Read a file from the filesystem |
| `write_file` | Write content to a file |
| `terminal` | Run a shell command |
| `memory` | Add / read / replace / remove persistent memory entries |
| `session_search` | Full-text search across past conversations (FTS5) |
| `skills_list` | List all available skills |
| `skill_view` | Load a skill's instructions by name |
| `delegate_task` | Spawn a parallel sub-agent for a decomposed subtask |

### MCP Tools

Any MCP server can be connected in `~/.garudust/config.yaml`:

```yaml
mcp_servers:
  - name: filesystem
    command: npx
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
  - name: postgres
    command: npx
    args: ["-y", "@modelcontextprotocol/server-postgres", "postgresql://localhost/mydb"]
```

The tools appear automatically in the agent's registry.

---

## Skills

Skills are reusable instruction sets loaded from `~/.garudust/skills/`. The agent sees them in every prompt and loads individual ones on demand.

```
~/.garudust/skills/
  git-workflow/SKILL.md
  daily-standup/SKILL.md
```

Minimal `SKILL.md`:

```markdown
---
name: git-workflow
description: Opinionated Git commit and PR workflow
version: 1.0.0
---

Always write conventional commits. Always run tests before pushing...
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      garudust-server                        │
│                                                             │
│  HTTP /chat ──┐                                             │
│  HTTP /stream ┤                                             │
│  Telegram ────┼──► GatewayHandler ──► Arc<Agent>           │
│  Discord ─────┤                            │                │
│  Webhook ─────┘                            ▼                │
│  Cron ─────────────────────────►  run_loop()                │
│                                       │        │            │
│                               Transport    ToolRegistry     │
│                           (Anthropic /   (web, file,        │
│                            OpenRouter /   terminal,         │
│                            Bedrock /      memory,           │
│                            Codex)         MCP, ...)         │
└─────────────────────────────────────────────────────────────┘
```

### Crate layout

```
crates/
  garudust-core        Shared traits & types — zero I/O, depended on by all
  garudust-transport   LLM adapters: Anthropic, OpenAI-compat, Codex, Bedrock
  garudust-tools       Tool registry + built-in toolsets
  garudust-memory      FileMemoryStore (markdown) + SessionDb (SQLite + FTS5)
  garudust-agent       Agent run loop, context compressor, prompt builder
  garudust-platforms   Platform adapters: Telegram, Discord, Webhook
  garudust-cron        Cron scheduler
  garudust-gateway     axum HTTP gateway, /health, /chat, /chat/stream

bin/
  garudust             CLI: TUI, one-shot, setup, doctor, config
  garudust-server      Headless: all platforms + HTTP + cron in one process
```

---

## Roadmap

Shipped:
- [x] Streaming SSE endpoint (`POST /chat/stream`)
- [x] MCP client — connect any MCP tool server
- [x] Session search (`session_search` tool, FTS5)
- [x] Slash commands in TUI (`/new`, `/model`, `/help`)
- [x] AWS Bedrock transport (Converse API)
- [x] OpenAI Responses API transport (Codex)
- [x] Docker + `docker-compose`

Up next:
- [x] `delegate_task` tool — spawn parallel sub-agents for decomposed work
- [x] Browser tool — CDP via `chromiumoxide`
- [ ] Slack and Matrix platform adapters
- [x] WebSocket transport (alternative to SSE)
- [x] Metrics endpoint (`/metrics`, Prometheus-compatible)
- [x] Rate limiting and request queuing in the HTTP gateway
- [x] Hot-reload skills and config without restart

---

## Contributing

Contributions are welcome! Garudust is designed to be easy to extend — adding a tool, transport, or platform adapter is typically under 100 lines and touches exactly one crate.

### Good first issues

- **New tool** — wrap any CLI or API as a `Tool` impl in `garudust-tools` (e.g. a `git` tool, an `image_gen` tool)
- **New platform** — implement `PlatformAdapter` for Slack, Matrix, or Signal
- **Improve TUI** — multi-line input, syntax highlighting, mouse support
- **Tests** — integration tests, property tests, snapshot tests

### Getting started

```bash
git clone https://github.com/ninenox/garudust
cd garudust
cargo build                          # build everything
cargo test --workspace               # run all tests
cargo clippy --workspace --all-targets \
  -- -W clippy::all -W clippy::pedantic   # lint (same as CI)
```

Read [CONTRIBUTING.md](CONTRIBUTING.md) for code guidelines, how to add a tool / platform / transport, commit conventions, and the full CI checklist.

---

## License

MIT — see [LICENSE](LICENSE).
