<div align="center">
  <img src="assets/logo.png" alt="Garudust" width="260"/>
</div>

# Garudust

**A self-hostable AI agent runtime written in Rust.**

Chat from your terminal, connect it to Telegram / Discord / Slack / Matrix, or call it over HTTP — all from a single binary.

[![CI](https://github.com/ninenox/garudust/actions/workflows/ci.yml/badge.svg)](https://github.com/ninenox/garudust/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Rust 1.75+](https://img.shields.io/badge/rust-1.75+-orange.svg)

---

## Why Garudust?

Most AI agent frameworks are Python, heavy, and slow to start. Garudust is:

- **~10 MB binary, < 20 ms cold start** — no Python runtime, no Docker required for local use
- **Swap providers with one env var** — Anthropic, OpenRouter, AWS Bedrock, OpenAI Responses API, or any OpenAI-compatible endpoint
- **Runs everywhere** — laptop TUI, headless server, Docker, Telegram, Discord, Slack, Matrix, HTTP
- **Composable** — every piece is a separate crate; add a tool, platform, or transport without touching anything else

---

## Quick Start

### 1. Docker (fastest)

```bash
# Cloud provider (OpenRouter, Anthropic, …)
echo "OPENROUTER_API_KEY=sk-or-..." > .env
docker compose up

# Or: local model via Ollama (no API key required)
echo "OLLAMA_BASE_URL=http://host.docker.internal:11434/v1" > .env
echo "GARUDUST_MODEL=llama3.2" >> .env
docker compose up
```

```bash
curl -X POST http://localhost:3000/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "what is 2+2?"}'
```

### 2. Build from source

**Prerequisites:** Rust 1.75+ and either an API key from [OpenRouter](https://openrouter.ai) / [Anthropic](https://console.anthropic.com), or a running [Ollama](https://ollama.com) instance.

```bash
git clone https://github.com/ninenox/garudust
cd garudust
cargo build --release
export PATH="$PATH:$(pwd)/target/release"

garudust setup   # pick provider, save API key
garudust         # launch interactive TUI
```

---

## CLI Usage

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

### Config commands

```bash
garudust setup                              # first-time wizard
garudust doctor                             # check API key, connectivity, DB
garudust config show                        # print active config
garudust config set model anthropic/claude-opus-4-7
garudust config set OPENROUTER_API_KEY sk-or-...
garudust config set ANTHROPIC_API_KEY  sk-ant-...
```

---

## Headless Server

`garudust-server` runs the HTTP gateway, all platform adapters, and cron jobs in one process.

```bash
garudust-server --anthropic-key sk-ant-... --port 3000
```

### HTTP API

```bash
# Blocking
curl -X POST http://localhost:3000/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "write a haiku about Rust"}'

# Streaming (Server-Sent Events)
curl -X POST http://localhost:3000/chat/stream \
  -H "Content-Type: application/json" \
  -d '{"message": "explain async/await in 3 sentences"}'

# WebSocket
# Connect: ws://localhost:3000/chat/ws
# Send:    {"message": "your task"}
# Receive: text chunks … then {"done":true}

# Health & metrics
curl http://localhost:3000/health
curl http://localhost:3000/metrics   # Prometheus-compatible
```

---

## Platform Adapters

Connect the agent to any messaging platform by setting the relevant env vars and starting `garudust-server`.

### Telegram

1. Create a bot via [@BotFather](https://t.me/botfather), copy the token.
2. Start the server:

```bash
TELEGRAM_TOKEN=123456:ABC... garudust-server --anthropic-key sk-ant-...
```

### Discord

1. Create an application at [discord.com/developers](https://discord.com/developers/applications).
2. Under **Bot**, enable **Message Content Intent** and copy the token.

```bash
DISCORD_TOKEN=Bot_... garudust-server --anthropic-key sk-ant-...
```

### Slack

1. Create a Slack app at [api.slack.com/apps](https://api.slack.com/apps).
2. Enable **Socket Mode** and generate an App-Level Token (`xapp-…`).
3. Add Bot Token Scopes: `chat:write`, `channels:history`, `im:history`.
4. Install to workspace and copy the Bot Token (`xoxb-…`).

```bash
SLACK_BOT_TOKEN=xoxb-... \
SLACK_APP_TOKEN=xapp-... \
garudust-server --anthropic-key sk-ant-...
```

### Matrix

Works with any Matrix homeserver (matrix.org, self-hosted Synapse/Dendrite, etc.).

```bash
MATRIX_HOMESERVER=https://matrix.org \
MATRIX_USER=@yourbot:matrix.org \
MATRIX_PASSWORD=secret \
garudust-server --anthropic-key sk-ant-...
```

### Webhook

Receives `POST /webhook`, runs the agent, and POSTs the reply to `callback_url`.

```bash
curl -X POST http://localhost:3001/webhook \
  -H "Content-Type: application/json" \
  -d '{"text":"summarise today","callback_url":"https://your-app/reply"}'
```

---

## LLM Providers

| Provider | How to select | Notes |
|----------|--------------|-------|
| Anthropic | Set `ANTHROPIC_API_KEY` | Direct Messages API |
| OpenRouter | Set `OPENROUTER_API_KEY` *(default)* | 200+ models |
| AWS Bedrock | Set `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` | Converse API, SigV4 |
| OpenAI Responses API | `garudust config set provider codex` | `/v1/responses` endpoint |
| Ollama | Set `OLLAMA_BASE_URL` | Local, no key required |
| vLLM | Set `VLLM_BASE_URL` | Local OpenAI-compatible server |
| Any OpenAI-compatible | Set `GARUDUST_BASE_URL` | OpenAI-compatible transport |

### Ollama (local, no key required)

```bash
# default: http://localhost:11434/v1
OLLAMA_BASE_URL=http://localhost:11434/v1
GARUDUST_MODEL=llama3.2
```

```bash
docker compose up        # or: garudust-server
curl -X POST http://localhost:3000/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "hello"}'
```

### vLLM (local OpenAI-compatible server)

```bash
VLLM_BASE_URL=http://localhost:8000/v1
VLLM_API_KEY=token-abc123          # only if vLLM started with --api-key
GARUDUST_MODEL=meta-llama/Llama-3.1-8B-Instruct
```

### AWS Bedrock

```bash
garudust config set provider bedrock
garudust config set model    anthropic.claude-3-5-sonnet-20241022-v2:0
```

---

## Built-in Tools

| Tool | Description |
|------|-------------|
| `web_fetch` | Fetch a URL (static pages) |
| `web_search` | Search via Brave Search API (`BRAVE_SEARCH_API_KEY`) |
| `browser` | Control Chrome/Chromium via CDP — navigate, click, type, screenshot, run JS |
| `read_file` | Read a file from the filesystem |
| `write_file` | Write a file to the filesystem |
| `terminal` | Run a shell command |
| `memory` | Persistent key-value memory (add / read / replace / remove) |
| `session_search` | Full-text search across past conversations (SQLite FTS5) |
| `delegate_task` | Spawn a parallel sub-agent for decomposed work |
| `skills_list` | List available skills |
| `skill_view` | Load a skill's instructions by name |

### MCP Tools

Connect any [MCP](https://modelcontextprotocol.io) server in `~/.garudust/config.yaml`:

```yaml
mcp_servers:
  - name: filesystem
    command: npx
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
  - name: postgres
    command: npx
    args: ["-y", "@modelcontextprotocol/server-postgres", "postgresql://localhost/mydb"]
```

Tools from connected MCP servers appear automatically in the agent's registry.

---

## Skills

Skills are reusable instruction sets stored in `~/.garudust/skills/`. They are read from disk on every invocation — edit a skill file and the next agent call picks up the change immediately.

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

## All Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `ANTHROPIC_API_KEY` | — | Anthropic key (auto-selects Anthropic transport) |
| `OPENROUTER_API_KEY` | — | OpenRouter key (default provider) |
| `OLLAMA_BASE_URL` | — | Ollama base URL — auto-selects Ollama transport, no key required |
| `VLLM_BASE_URL` | — | vLLM base URL — auto-selects vLLM transport |
| `VLLM_API_KEY` | — | vLLM API key (optional, only if server requires it) |
| `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` | — | Bedrock credentials |
| `BRAVE_SEARCH_API_KEY` | — | Enables `web_search` tool |
| `GARUDUST_MODEL` | `anthropic/claude-sonnet-4-6` | Model identifier |
| `GARUDUST_PORT` | `3000` | HTTP gateway port |
| `GARUDUST_WEBHOOK_PORT` | `3001` | Webhook adapter port (`0` = disabled) |
| `GARUDUST_BASE_URL` | — | Override LLM base URL (any OpenAI-compatible) |
| `GARUDUST_API_KEY` | — | Bearer token for `/chat*` endpoints (recommended in production) |
| `GARUDUST_APPROVAL_MODE` | `smart` | Command approval: `auto` \| `smart` \| `deny` |
| `GARUDUST_RATE_LIMIT` | — | Per-IP rate limit in requests/minute |
| `TELEGRAM_TOKEN` | — | Telegram bot token |
| `DISCORD_TOKEN` | — | Discord bot token |
| `SLACK_BOT_TOKEN` | — | Slack bot token (`xoxb-…`) |
| `SLACK_APP_TOKEN` | — | Slack app token for Socket Mode (`xapp-…`) |
| `MATRIX_HOMESERVER` | — | Matrix homeserver URL |
| `MATRIX_USER` | — | Matrix username (`@bot:matrix.org`) |
| `MATRIX_PASSWORD` | — | Matrix password |
| `GARUDUST_CRON_JOBS` | — | Comma-separated `"cron_expr=task"` pairs |
| `RUST_LOG` | `info` | Log level (`debug` for verbose) |

### Cron jobs

```bash
GARUDUST_CRON_JOBS="0 9 * * *=Write a morning briefing and save to ~/briefing.md" \
garudust-server --anthropic-key sk-ant-...
```

---

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                        garudust-server                           │
│                                                                  │
│  HTTP /chat ────┐                                                │
│  HTTP /stream   │                                                │
│  WebSocket ─────┼──► GatewayHandler ──► ArcSwap<Agent>          │
│  Telegram       │                            │                   │
│  Discord        │                            ▼                   │
│  Slack ─────────┘                       run_loop()               │
│  Matrix                                  │         │             │
│  Cron ──────────────────────────►   Transport   ToolRegistry     │
│                                    (Anthropic    (web, browser,  │
│                                     OpenRouter   file, terminal, │
│                                     Bedrock      memory, MCP,    │
│                                     Codex        delegate, ...)  │
│                                     Ollama                       │
│                                     vLLM)                        │
└──────────────────────────────────────────────────────────────────┘
```

### Crate layout

```
crates/
  garudust-core        Shared traits & types — zero I/O
  garudust-transport   LLM adapters: Anthropic, OpenAI-compat, Codex, Bedrock, Ollama, vLLM
  garudust-tools       Tool registry + built-in toolsets (web, browser, file, …)
  garudust-memory      FileMemoryStore (markdown) + SessionDb (SQLite + FTS5)
  garudust-agent       Agent run loop, context compressor, prompt builder
  garudust-platforms   Telegram, Discord, Slack, Matrix, Webhook
  garudust-cron        Cron scheduler
  garudust-gateway     axum HTTP gateway — /chat, /chat/stream, /chat/ws, /metrics

bin/
  garudust             CLI: interactive TUI, one-shot, setup, doctor, config
  garudust-server      Headless: all platforms + HTTP + cron in one process
```

---

## Contributing

Garudust is designed to be easy to extend — adding a tool, transport, or platform adapter typically touches exactly one crate and takes under 100 lines.

### Good first issues

- **New tool** — wrap any CLI or API as a `Tool` impl in `garudust-tools`
- **New platform** — implement `PlatformAdapter` (e.g. Signal, LINE, WhatsApp)
- **Improve TUI** — multi-line input, syntax highlighting, mouse support
- **Tests** — integration tests, property tests, snapshot tests

### Getting started

```bash
git clone https://github.com/ninenox/garudust
cd garudust
cargo build                   # build everything
cargo test --workspace        # run all tests
cargo clippy --workspace --all-targets \
  -- -W clippy::all -W clippy::pedantic   # lint (same as CI)
```

Read [CONTRIBUTING.md](CONTRIBUTING.md) for code guidelines, commit conventions, and the full CI checklist.

---

## License

MIT — see [LICENSE](LICENSE).
