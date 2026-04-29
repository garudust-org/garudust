<div align="center">
  <img src="assets/logo.png" alt="Garudust" width="260"/>

  <a href="README.md"><img src="https://img.shields.io/badge/🇺🇸-English-blue?style=flat-square" alt="English"/></a>
  <a href="docs/i18n/th/README.md"><img src="https://img.shields.io/badge/🇹🇭-ภาษาไทย-red?style=flat-square" alt="ภาษาไทย"/></a>
  <a href="docs/i18n/zh/README.md"><img src="https://img.shields.io/badge/🇨🇳-简体中文-yellow?style=flat-square" alt="简体中文"/></a>
</div>

# Garudust Agent

[![CI](https://github.com/garudust-org/garudust-agent/actions/workflows/ci.yml/badge.svg)](https://github.com/garudust-org/garudust-agent/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/garudust-org/garudust-agent)](https://github.com/garudust-org/garudust-agent/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Rust 1.75+](https://img.shields.io/badge/rust-1.75+-orange.svg)

**A self-hostable AI agent runtime written in Rust.**

Chat from your terminal, connect it to Telegram / Discord / Slack / Matrix, or call it over HTTP — all from a single binary.

<div align="center">
  <img src="assets/demo.svg" alt="Garudust demo"/>
</div>

---

## Why Garudust?

Most AI agent frameworks are Python, heavy, and slow to start. Garudust is:

- **~10 MB binary, < 20 ms cold start** — no Python runtime, no Docker required for local use
- **Swap providers with one env var** — Anthropic, OpenRouter, AWS Bedrock, Ollama, vLLM, or any OpenAI-compatible endpoint
- **Runs everywhere** — laptop TUI, headless server, Docker, Telegram, Discord, Slack, Matrix, HTTP
- **Composable** — every piece is a separate crate; add a tool, platform, or transport without touching anything else

---

## Install

### Pre-built binaries (recommended)

Download from [**GitHub Releases**](https://github.com/garudust-org/garudust-agent/releases/latest) — no Rust required:

| Platform | File |
|----------|------|
| macOS Apple Silicon | `garudust-*-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `garudust-*-x86_64-apple-darwin.tar.gz` |
| Linux x86_64 | `garudust-*-x86_64-unknown-linux-musl.tar.gz` |
| Linux ARM64 | `garudust-*-aarch64-unknown-linux-musl.tar.gz` |
| Windows | `garudust-*-x86_64-pc-windows-msvc.zip` |

```bash
tar -xzf garudust-*.tar.gz
sudo mv garudust garudust-server /usr/local/bin/
```

### Build from source

Requires Rust 1.75+:

```bash
git clone https://github.com/garudust-org/garudust-agent
cd garudust
cargo build --release
export PATH="$PATH:$(pwd)/target/release"
```

---

## Quick Start

### 1. Configure and chat

```bash
garudust setup   # pick provider (OpenRouter / Anthropic / vLLM / Ollama / custom) + save key
garudust         # launch interactive TUI
```

### 2. Docker (server mode)

```bash
# Cloud provider
echo "OPENROUTER_API_KEY=sk-or-..." > .env
docker compose up

# Local model via Ollama
echo "OLLAMA_BASE_URL=http://host.docker.internal:11434" > .env
echo "GARUDUST_MODEL=llama3.2" >> .env
docker compose up
```

```bash
curl -X POST http://localhost:3000/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "what is 2+2?"}'
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
garudust setup                              # first-time wizard (Quick or Full mode)
garudust doctor                             # check API key, connectivity, DB
garudust config show                        # print active config
garudust model                              # show current model, prompt for new
garudust model anthropic/claude-opus-4-7   # switch model directly
garudust config set OPENROUTER_API_KEY sk-or-...
garudust config set ANTHROPIC_API_KEY sk-ant-...
garudust config set VLLM_BASE_URL http://localhost:8000/v1
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

# WebSocket: ws://localhost:3000/chat/ws
# Send: {"message": "your task"}  Receive: text chunks … then {"done":true}

# Health & metrics
curl http://localhost:3000/health
curl http://localhost:3000/metrics   # Prometheus-compatible
```

---

## Platform Adapters

Set the relevant env vars and start `garudust-server`. Every adapter can run together in the same process.

| Platform | Required env vars |
|----------|-------------------|
| Telegram | `TELEGRAM_TOKEN` |
| Discord | `DISCORD_TOKEN` |
| Slack | `SLACK_BOT_TOKEN`, `SLACK_APP_TOKEN` |
| Matrix | `MATRIX_HOMESERVER`, `MATRIX_USER`, `MATRIX_PASSWORD` |
| Webhook | always-on at `POST /webhook` — no token needed |

**Telegram** — create a bot via [@BotFather](https://t.me/botfather), copy the token.

**Discord** — create an app at [discord.com/developers](https://discord.com/developers/applications), enable **Message Content Intent** under Bot, copy the token.

**Slack** — create an app at [api.slack.com/apps](https://api.slack.com/apps), enable **Socket Mode**, add scopes `chat:write channels:history im:history`, install to workspace.

**Matrix** — works with any homeserver (matrix.org, Synapse, Dendrite, etc.).

```bash
TELEGRAM_TOKEN=123:ABC \
SLACK_BOT_TOKEN=xoxb-... \
SLACK_APP_TOKEN=xapp-... \
garudust-server --anthropic-key sk-ant-...
```

---

## LLM Providers

| Provider | How to select | Notes |
|----------|--------------|-------|
| Anthropic | Set `ANTHROPIC_API_KEY` | Direct Messages API |
| OpenRouter | Set `OPENROUTER_API_KEY` *(default)* | 200+ models |
| AWS Bedrock | Set `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` | Converse API, SigV4 |
| OpenAI Responses | `garudust config set provider codex` | `/v1/responses` endpoint |
| Ollama | Set `OLLAMA_BASE_URL` | Local, no key required |
| vLLM | Set `VLLM_BASE_URL` | Local OpenAI-compatible server |
| Any OpenAI-compat | Set `GARUDUST_BASE_URL` | Generic transport |

```bash
# Ollama (local, no key)
OLLAMA_BASE_URL=http://localhost:11434
GARUDUST_MODEL=llama3.2

# vLLM
VLLM_BASE_URL=http://localhost:8000/v1
VLLM_API_KEY=token-abc123          # only if server requires --api-key
GARUDUST_MODEL=meta-llama/Llama-3.1-8B-Instruct

# AWS Bedrock
garudust config set provider bedrock
garudust config set model anthropic.claude-3-5-sonnet-20241022-v2:0
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

---

## Skills

Skills are reusable instruction sets stored in `~/.garudust/skills/`. They are loaded from disk on every invocation — edit a file and the next call picks up the change immediately.

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
| `OLLAMA_BASE_URL` | — | Ollama base URL — auto-selects Ollama, no key needed |
| `VLLM_BASE_URL` | — | vLLM base URL — auto-selects vLLM transport |
| `VLLM_API_KEY` | — | vLLM API key (optional) |
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
  garudust             CLI: interactive TUI, one-shot, setup, doctor, config, model
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

```bash
git clone https://github.com/garudust-org/garudust-agent
cd garudust
cargo build
cargo test --workspace
cargo clippy --workspace --all-targets -- -W clippy::all -W clippy::pedantic
```

Read [CONTRIBUTING.md](CONTRIBUTING.md) for code guidelines, commit conventions, and the full CI checklist.

---

## License

MIT — see [LICENSE](LICENSE).
