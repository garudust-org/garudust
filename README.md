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

**A self-hostable, self-improving AI agent runtime written in Rust.**

Chat from your terminal, connect it to Telegram / Discord / Slack / Matrix / LINE, or call it over HTTP — all from a single binary. It remembers what you teach it, speaks your language, and gets smarter with every session.

<div align="center">
  <img src="assets/demo.svg" alt="Garudust demo"/>
</div>

---

## Why Garudust?

- **~10 MB binary, < 20 ms cold start** — single statically-linked binary, no runtime dependencies for local use
- **Self-improving** — learns your preferences, saves reusable workflows as skills, and corrects itself without being told twice
- **Speaks your language** — detects Thai, Chinese, Japanese, Arabic, Korean, and more automatically; no configuration needed
- **Swap providers with one env var** — Anthropic, OpenRouter, AWS Bedrock, Ollama, vLLM, or any OpenAI-compatible endpoint
- **Secure by design** — Docker sandbox, hardline command blocks, memory-poisoning protection, and automatic secret redaction from tool output
- **Runs everywhere** — laptop TUI, headless server, Docker, Telegram, Discord, Slack, Matrix, LINE, HTTP
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
cd garudust-agent
cargo build --release
export PATH="$PATH:$(pwd)/target/release"
```

---

## Quick Start

```bash
garudust setup   # first-time wizard — pick provider, save API key
```

| | **1 — TUI** | **2 — One-shot** | **3 — Server / Docker** |
|---|---|---|---|
| **Command** | `garudust` | `garudust "task"` | `garudust-server` |
| **Use when** | Personal interactive chat | Script / pipe / CI | Bots, API, cron jobs |
| **Interface** | Terminal UI | stdout + exit code | HTTP + chat platforms |

### 1 — Interactive TUI

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

### 2 — One-shot

```bash
garudust "summarise the git log from the last 7 days into a changelog"
```

Output goes to stdout. Exit code is 0 on success. Pipe-friendly.

### 3 — Server / Docker

```bash
# Minimal
garudust-server --port 3000

# With Docker
echo "OPENROUTER_API_KEY=sk-or-..." > .env
docker compose up

# Production: sandbox + Telegram bot + daily cron
GARUDUST_TERMINAL_SANDBOX=docker \
GARUDUST_API_KEY=my-secret-token \
TELEGRAM_TOKEN=123:ABC \
GARUDUST_CRON_JOBS="0 9 * * *=Post a morning briefing to Telegram" \
GARUDUST_MEMORY_CRON="0 3 * * *" \
garudust-server --port 3000 --approval-mode smart
```

---

## CLI Reference

```bash
garudust setup                              # first-time wizard
garudust doctor                             # check API key, connectivity, DB
garudust config show                        # print active config
garudust model                              # show current model, prompt for new
garudust model anthropic/claude-opus-4-7   # switch model directly
garudust config set ANTHROPIC_API_KEY sk-ant-...
garudust config set VLLM_BASE_URL http://localhost:8000/v1
```

---

## Configuration

All persistent settings live in `~/.garudust/config.yaml`. Secrets and tokens live in `~/.garudust/.env` — run `garudust setup` to configure them interactively. Both files are loaded securely at startup and never forwarded to subprocesses.

### `~/.garudust/config.yaml`

```yaml
model: anthropic/claude-sonnet-4-6   # model identifier
provider: anthropic                  # auto-detected from API key if omitted

security:
  terminal_sandbox: docker           # none (default) | docker
  terminal_sandbox_image: ubuntu:24.04
  terminal_sandbox_opts:
    - "--network=none"               # cut outbound network access inside container
    - "--memory=512m"                # cap memory

nudge_interval: 5                    # memory-save reminder every N iterations (0 = off)

mcp_servers:
  - name: filesystem
    command: npx
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
  - name: postgres
    command: npx
    args: ["-y", "@modelcontextprotocol/server-postgres", "postgresql://localhost/mydb"]
```

## Security

Garudust is designed to be safe when the agent has access to real tools — filesystem, terminal, and the web.

### Terminal sandbox

When `terminal_sandbox: docker` is set, every shell command runs inside an isolated container with hardened defaults: `--cap-drop ALL`, `--security-opt no-new-privileges:true`, `--pids-limit 256`, and an ephemeral `/tmp`. The current working directory is mounted at `/workspace` so file operations still work.

> **Note:** Docker must be installed and running. A clear error is raised at startup and at tool-call time if it is missing.

### Hardline command blocks

The following patterns are blocked unconditionally, regardless of approval mode or sandbox:

| Pattern | Example |
|---------|---------|
| Recursive root filesystem deletion | `rm -rf /`, `rm -rf /*` |
| Filesystem format | `mkfs`, `mkfs.ext4 /dev/sda1` |
| Fork bomb | `:(){ :|:& };:` |
| Writing to raw block devices | `dd of=/dev/sda`, `cat > /dev/nvme0n1` |
| System shutdown / reboot | `shutdown`, `reboot`, `halt`, `systemctl poweroff` |
| Writes to credential paths | `~/.ssh/authorized_keys`, `~/.aws/credentials`, `~/.bashrc` |

### Approval modes

| Mode | Behaviour |
|------|-----------|
| `smart` *(default)* | Approves all tools; system prompt's constitutional constraints are the primary gate; every destructive call is audit-logged |
| `auto` | Same as `smart` — use in trusted automation pipelines where logging overhead matters |
| `deny` | Blocks all destructive tool calls unconditionally — ideal for read-only agents |

Set via `GARUDUST_APPROVAL_MODE` or `--approval-mode`.

### Memory protection

Memory entries retrieved from previous sessions are wrapped in `<untrusted_memory>` tags so the model treats them as user-controlled data rather than trusted instructions. This prevents memory-poisoning attacks where a malicious tool output plants a jailbreak into persistent memory. Write-time validation also rejects entries that contain XML control tags.

### Output redaction

API keys and secrets loaded at startup are automatically scrubbed from terminal command output before it reaches the model. Output is also truncated to 50 KB (40% head + 60% tail) to prevent context flooding.

---

## Memory & Self-Improvement

Garudust remembers facts across sessions and gets smarter the more you use it.

### How memory works

The agent automatically saves durable knowledge to `~/.garudust/memory/` — user preferences, project conventions, corrections you make to its behaviour:

```
You: always format JSON with 2-space indent
Agent: [saves to memory] Got it — I'll use 2-space indent for JSON from now on.
```

On the next session, that preference is already loaded. You never need to repeat yourself.

A built-in nudge fires every few iterations during long tasks, prompting the agent to persist any new facts before the session ends. Configure the interval in `config.yaml` with `nudge_interval` (0 = off).

### What gets saved

| Category | Examples |
|----------|---------|
| Preferences | output format, language, tone, tool choices |
| Project details | paths, configs, conventions, known quirks |
| Corrections | anything you tell the agent to stop doing — saved immediately |

The agent does **not** save session logs, task progress, or one-off details — only facts that will matter in future sessions.

---

## Skills

Skills are reusable instruction sets the agent loads before acting. They live in `~/.garudust/skills/` and are hot-reloaded on every call — edit a file and the next message picks up the change immediately.

```
~/.garudust/skills/
  git-workflow/SKILL.md
  daily-standup/SKILL.md
  rust-code-review/SKILL.md
```

### Proactive skill loading

Before processing any message, the agent scans all available skills. If a skill is relevant — even partially — it calls `skill_view` to load the full instructions before proceeding. This ensures established workflows are always followed, regardless of the language you write in.

### Creating skills

The agent creates skills automatically when it discovers a multi-step workflow:

```
You: write a skill for reviewing Rust PRs
Agent: [calls write_skill] Saved skill 'rust-code-review' to ~/.garudust/skills/rust-code-review/SKILL.md
```

You can also create them directly with the `write_skill` tool, or write `SKILL.md` files by hand.

Minimal `SKILL.md`:

```markdown
---
name: git-workflow
description: Opinionated Git commit and PR workflow
version: 1.0.0
---

Always write conventional commits. Always run tests before pushing.
Open a draft PR first, then mark ready when CI is green.
```

### Updating skills

If the agent finds a skill's steps are outdated or wrong during a task, it patches the file immediately — no need to ask. Skills stay accurate automatically.

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

Set the relevant tokens in `~/.garudust/.env` and start `garudust-server`. Every adapter runs together in the same process.

| Platform | Required tokens |
|----------|-----------------|
| Telegram | `TELEGRAM_TOKEN` |
| Discord | `DISCORD_TOKEN` |
| Slack | `SLACK_BOT_TOKEN`, `SLACK_APP_TOKEN` |
| Matrix | `MATRIX_HOMESERVER`, `MATRIX_USER`, `MATRIX_PASSWORD` |
| LINE | `LINE_CHANNEL_TOKEN`, `LINE_CHANNEL_SECRET` |
| Webhook | always-on at `POST /webhook` — no token needed |

**Telegram** — create a bot via [@BotFather](https://t.me/botfather), copy the token.

**Discord** — create an app at [discord.com/developers](https://discord.com/developers/applications), enable **Message Content Intent** under Bot, copy the token.

**Slack** — create an app at [api.slack.com/apps](https://api.slack.com/apps), enable **Socket Mode**, add scopes `chat:write channels:history im:history`, install to workspace.

**Matrix** — works with any homeserver (matrix.org, Synapse, Dendrite, etc.).

**LINE** — create a Messaging API channel at [developers.line.biz](https://developers.line.biz/console/), copy the **Channel access token** and **Channel secret**, then set `GARUDUST_LINE_PORT` (default `3002`) and point the webhook URL in LINE console to `https://your-host:3002/line`.

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

Set the relevant key in `~/.garudust/.env`, then switch models with `garudust model` or by setting `GARUDUST_MODEL`.

---

## Built-in Tools

| Tool | Description |
|------|-------------|
| `web_fetch` | Fetch a URL (static pages) |
| `web_search` | Search via Brave Search API (`BRAVE_SEARCH_API_KEY`) |
| `browser` | Control Chrome/Chromium via CDP — navigate, click, type, screenshot, run JS |
| `read_file` | Read a file from the filesystem |
| `write_file` | Write a file to the filesystem; sensitive credential paths are always blocked |
| `terminal` | Run a shell command; sandboxed in Docker when `terminal_sandbox: docker` is set |
| `memory` | Persistent key-value memory (add / read / replace / remove) |
| `user_profile` | Read and update the persistent user profile |
| `session_search` | Full-text search across past conversations (SQLite FTS5) |
| `delegate_task` | Spawn a parallel sub-agent for decomposed work |
| `skills_list` | List available skills |
| `skill_view` | Load a skill's full instructions by name |
| `write_skill` | Create or update a skill in `~/.garudust/skills/` |

**MCP tools** — connect any [MCP](https://modelcontextprotocol.io) server by adding it to the `mcp_servers` list in `config.yaml` (see Configuration).

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
│  LINE                                                             │
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
  garudust-platforms   Telegram, Discord, Slack, Matrix, LINE, Webhook
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
- **New platform** — implement `PlatformAdapter` (e.g. Signal, WhatsApp)
- **Improve TUI** — multi-line input, syntax highlighting, mouse support
- **Tests** — integration tests, property tests, snapshot tests

```bash
git clone https://github.com/garudust-org/garudust-agent
cd garudust-agent
cargo build
cargo test --workspace
cargo clippy --workspace --all-targets -- -W clippy::all -W clippy::pedantic
```

Read [CONTRIBUTING.md](CONTRIBUTING.md) for code guidelines, commit conventions, and the full CI checklist.

---

## License

MIT — see [LICENSE](LICENSE).
