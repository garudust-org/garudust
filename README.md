<div align="center">
  <img src="assets/logo-agent.jpg" alt="Garudust"/>

  <a href="README.md"><img src="https://img.shields.io/badge/🇺🇸-English-blue?style=flat-square" alt="English"/></a>
  <a href="docs/i18n/th/README.md"><img src="https://img.shields.io/badge/🇹🇭-ภาษาไทย-red?style=flat-square" alt="ภาษาไทย"/></a>
  <a href="docs/i18n/zh/README.md"><img src="https://img.shields.io/badge/🇨🇳-简体中文-yellow?style=flat-square" alt="简体中文"/></a>
</div>

# Garudust Agent

[![CI](https://github.com/garudust-org/garudust-agent/actions/workflows/ci.yml/badge.svg)](https://github.com/garudust-org/garudust-agent/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/garudust-agent.svg)](https://crates.io/crates/garudust-agent)
[![Downloads](https://img.shields.io/crates/d/garudust-agent.svg)](https://crates.io/crates/garudust-agent)
[![Release](https://img.shields.io/github/v/release/garudust-org/garudust-agent)](https://github.com/garudust-org/garudust-agent/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Rust 1.87+](https://img.shields.io/badge/rust-1.87+-orange.svg)
[![Discord](https://img.shields.io/badge/Discord-Join%20Community-5865F2?logo=discord&logoColor=white&style=flat-square)](https://discord.com/channels/1501414298449088745/1501414298893942877)

**Your AI agent. Your server. Your rules.**

A self-improving AI agent runtime written in Rust — delivered as a single ~10 MB binary with no runtime dependencies. One binary handles everything: chat in the terminal, reply across multiple platforms (Telegram, Discord, Slack, LINE, WhatsApp), or expose a REST + WebSocket API. Extend it instantly via the Tool Hub or drop a YAML file to add your own tools. Connect any MCP server, or let the agent write and refine its own reusable skills. No telemetry, no lock-in — your data goes only to the LLM provider you choose.

### Demo

<div align="center">
  <img src="assets/demo.svg" alt="Garudust demo"/>
</div>

---

## Why Garudust?

- **~10 MB binary, < 20 ms cold start** — single statically-linked binary, no runtime dependencies for local use
- **Self-improving** — learns your preferences, saves reusable workflows as skills, and corrects itself without being told twice
- **agentskills.io compatible** — install skills from the [agentskills.io](https://agentskills.io) hub or any GitHub repo with one command; `allowed-tools`, version pinning, and scripts work out of the box
- **One-command Tool Hub** — browse and install community-built script tools instantly with `garudust tool install <name>`; no manual folder setup, no runtime wrangling
- **Speaks your language** — detects Thai, Chinese, Japanese, Arabic, Korean, and more automatically; no configuration needed
- **Swap providers with one env var** — Anthropic, OpenRouter, AWS Bedrock, Ollama, vLLM, ThaiLLM, or any OpenAI-compatible endpoint
- **Secure by design** — Docker sandbox, hardline command blocks, memory-poisoning protection, and automatic secret redaction from tool output
- **Runs everywhere** — laptop TUI, headless server, Docker, Telegram, Discord, Slack, Matrix, LINE, WhatsApp, HTTP
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

Requires Rust 1.87+:

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
garudust         # start agent chat with TUI
```

### 1 — Interactive TUI

```bash
garudust
```

<div align="center">
  <img src="assets/demo-tui.png" alt="Garudust TUI" width="800"/>
</div>

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

### 3 — Server / Docker with Platforms

```bash
# Minimal
garudust-server --port 3000

# With Docker
echo "OPENROUTER_API_KEY=sk-or-..." > .env
docker compose up

# Production: sandbox + LINE bot + daily cron
# 1. Put secrets in ~/.garudust/.env:  LINE_CHANNEL_TOKEN, LINE_CHANNEL_SECRET
# 2. Enable the LINE adapter in ~/.garudust/config.yaml:
#      platforms:
#        line: { enabled: true, port: 3002, webhook_path: /line }
GARUDUST_TERMINAL_SANDBOX=docker \
GARUDUST_API_KEY=my-secret-token \
GARUDUST_CRON_JOBS="0 9 * * *=Post a morning briefing to LINE" \
GARUDUST_MEMORY_CRON="0 3 * * *" \
garudust-server --port 3000 --approval-mode smart

# Expose LINE webhook via ngrok (development)
ngrok http 3002
# Webhook URL: https://xxxx.ngrok-free.app/line  ← paste this into LINE Developers Console
```

<div align="center">
  <img src="assets/demo-line.jpg" alt="LINE Demo" width="420"/>
</div>

---

## CLI Reference

```bash
garudust setup                              # first-time wizard
garudust doctor                             # check API key, connectivity, DB
garudust config show                        # print active config
garudust model                              # show current model, prompt for new
garudust model anthropic/claude-opus-4-7   # switch model directly
garudust config set ANTHROPIC_API_KEY sk-ant-...
garudust config set provider vllm
garudust config set base_url http://localhost:8000/v1
```

---

## Configuration

Non-secret settings live in `~/.garudust/config.yaml`. API keys and tokens live in `~/.garudust/.env` — run `garudust setup` to configure them interactively. Both files are loaded securely at startup and never forwarded to subprocesses.

### `~/.garudust/config.yaml`

```yaml
# Model and provider — not secrets, so they live here (not in .env)
model: anthropic/claude-sonnet-4-6   # model identifier
provider: anthropic                  # anthropic | openrouter | vllm | ollama | thaillm
base_url: https://your-vllm-host/v1  # required for vllm / ollama / any OpenAI-compat

security:
  terminal_sandbox: docker           # none (default) | docker
  terminal_sandbox_image: ubuntu:24.04
  terminal_sandbox_opts:
    - "--network=none"               # cut outbound network access inside container
    - "--memory=512m"                # cap memory

nudge_interval: 5                    # memory-save reminder every N iterations (0 = off)

# Disable entire toolsets (reduces context usage on small-context models)
# Available: web, files, terminal, memory, skills, agent, browser, git, notes, json, mcp
disabled_toolsets: [browser, git, notes]

# Disable individual tools without removing their whole toolset
disabled_tools: [image_read, pdf_read, session_search]

# For small-context models (e.g. 27K): set context_window so the agent
# automatically caps output tokens and retries on overflow
context_window: 27168

mcp_servers:
  - name: filesystem
    command: npx
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
  - name: postgres
    command: npx
    args: ["-y", "@modelcontextprotocol/server-postgres", "postgresql://localhost/mydb"]

# HTTP gateway settings — `--port` / `GARUDUST_PORT` override.
server:
  port: 3000

# Cron — recurring agent tasks plus memory housekeeping. `--cron-jobs` /
# `--memory-cron` / `--memory-expiry-cron` (and matching env vars) override.
cron:
  jobs:
    - schedule: "0 9 * * *"
      task: "Write a morning briefing and save to ~/briefing.md"
  memory_consolidation: "0 3 * * *"   # null/omitted = disabled
  memory_expiry: "0 4 * * *"           # null/omitted = disabled
```

### Platform Setup

#### Telegram bot

```bash
# ~/.garudust/.env
ANTHROPIC_API_KEY=sk-ant-...
TELEGRAM_TOKEN=123456789:AAFxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx

# start
garudust-server --telegram-token $TELEGRAM_TOKEN --anthropic-key $ANTHROPIC_API_KEY
```

#### LINE Messaging API

Webhook-based adapters (LINE, WhatsApp, generic webhook) are configured in `~/.garudust/config.yaml` under `platforms.*`; secrets remain in `~/.garudust/.env`.

```bash
# ~/.garudust/.env
OPENROUTER_API_KEY=sk-or-...
LINE_CHANNEL_TOKEN=<channel-access-token>
LINE_CHANNEL_SECRET=<32-char-hex-secret>
```

```yaml
# ~/.garudust/config.yaml
platforms:
  line:
    enabled: true
    port: 3002
    webhook_path: /line   # webhook receives at https://your-host:3002/line
```

```bash
garudust-server --port 3000
```

#### WhatsApp Business

```bash
# ~/.garudust/.env
ANTHROPIC_API_KEY=sk-ant-...
WHATSAPP_ACCESS_TOKEN=EAAxxxxxxx
WHATSAPP_PHONE_NUMBER_ID=123456789012345
WHATSAPP_VERIFY_TOKEN=my_verify_token
WHATSAPP_APP_SECRET=<32-char-hex-secret>   # optional — skips HMAC check if empty
```

```yaml
# ~/.garudust/config.yaml
platforms:
  whatsapp:
    enabled: true
    port: 3003
    webhook_path: /whatsapp
```

```bash
garudust-server --port 3000
```

#### Multi-platform (Telegram + LINE + WhatsApp + HTTP webhook)

All adapters run in the same process. Secrets in `.env`, enable/port/path in `config.yaml`. Platforms with `enabled: false` (or missing tokens) are silently skipped.

```bash
# ~/.garudust/.env
ANTHROPIC_API_KEY=sk-ant-...
TELEGRAM_TOKEN=123456789:AAFxxx
LINE_CHANNEL_TOKEN=<token>
LINE_CHANNEL_SECRET=<secret>
WHATSAPP_ACCESS_TOKEN=EAAxxx
WHATSAPP_PHONE_NUMBER_ID=123456789012345
WHATSAPP_VERIFY_TOKEN=my_verify_token
```

```yaml
# ~/.garudust/config.yaml — enable webhook-based adapters
platforms:
  webhook:
    enabled: true
    port: 3001
    webhook_path: /webhook
  line:
    enabled: true
    port: 3002
    webhook_path: /line
  whatsapp:
    enabled: true
    port: 3003
    webhook_path: /whatsapp
```

```bash
garudust-server --port 3000
```

> **Tip:** Use `garudust setup` (mode 2 — Full) for an interactive wizard that writes `~/.garudust/.env` and the matching `platforms.*` blocks in `~/.garudust/config.yaml` for you.

## Security

### Terminal sandbox

Set `terminal_sandbox: docker` in `config.yaml` to run every shell command inside an isolated container (`--cap-drop ALL`, `--pids-limit 256`, working directory mounted at `/workspace`). Requires Docker.

### Hardline command blocks

Blocked unconditionally, regardless of approval mode:

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
| `smart` *(default)* | All tools allowed; constitutional constraints are the primary gate; destructive calls are audit-logged |
| `auto` | Same as `smart` — for trusted automation pipelines |
| `deny` | Blocks all destructive calls — for read-only agents |

Set via `GARUDUST_APPROVAL_MODE` or `--approval-mode`.

Memory entries from previous sessions are wrapped in `<untrusted_memory>` tags to prevent memory-poisoning attacks. API keys are scrubbed from tool output automatically; output is truncated to 50 KB to prevent context flooding.

---

## Memory & Self-Improvement

The agent saves durable knowledge to `~/.garudust/memory/` and loads it at the start of every session — you never need to repeat yourself:

```
You: always format JSON with 2-space indent
Agent: [saves to memory] Got it — I'll use 2-space indent for JSON from now on.
```

| Category | Examples |
|----------|---------|
| Preferences | output format, language, tone, tool choices |
| Project details | paths, configs, conventions, known quirks |
| Corrections | anything you tell the agent to stop doing — saved immediately |

Configure the memory-save nudge interval with `nudge_interval` in `config.yaml` (0 = off).

---

## Skills

Reusable instruction sets stored in `~/.garudust/skills/`, hot-reloaded on every call.

```
~/.garudust/skills/
  git-workflow/SKILL.md
  daily-standup/SKILL.md
  rust-code-review/SKILL.md
```

The agent scans all skills before every message and loads any that are relevant. It creates and patches skill files automatically when it discovers or corrects a workflow.

Garudust is compatible with the [agentskills.io](https://agentskills.io) open standard — skills load and run without modification, including `allowed-tools` restrictions and `scripts/` execution.

Install skills from the agentskills.io hub or any GitHub repo with one command:

```bash
# From GitHub (owner/repo/path)
garudust skill install agentskills-org/hub/git-workflow

# From a direct URL
garudust skill install https://example.com/skills/my-skill/SKILL.md

# From a well-known endpoint
garudust skill install well-known:https://example.com --name my-skill

garudust skill list                      # show installed skills
garudust skill uninstall git-workflow    # remove a skill
```

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

<div align="center">
  <a href="https://core.telegram.org/bots"><img src="https://img.shields.io/badge/Telegram-2CA5E0?logo=telegram&logoColor=white&style=for-the-badge" alt="Telegram"/></a>
  <a href="https://discord.com/developers/applications"><img src="https://img.shields.io/badge/Discord-5865F2?logo=discord&logoColor=white&style=for-the-badge" alt="Discord"/></a>
  <a href="https://api.slack.com/apps"><img src="https://img.shields.io/badge/Slack-4A154B?logo=slack&logoColor=white&style=for-the-badge" alt="Slack"/></a>
  <a href="https://matrix.org"><img src="https://img.shields.io/badge/Matrix-000000?logo=matrix&logoColor=white&style=for-the-badge" alt="Matrix"/></a>
  <a href="https://developers.line.biz/console/"><img src="https://img.shields.io/badge/LINE-00C300?logo=line&logoColor=white&style=for-the-badge" alt="LINE"/></a>
  <a href="https://developers.facebook.com/docs/whatsapp/cloud-api"><img src="https://img.shields.io/badge/WhatsApp-25D366?logo=whatsapp&logoColor=white&style=for-the-badge" alt="WhatsApp"/></a>
  <img src="https://img.shields.io/badge/Webhook-6E7681?style=for-the-badge" alt="Webhook"/>
</div>

Set the relevant tokens in `~/.garudust/.env` and start `garudust-server`. Every adapter runs together in the same process.

| Platform | Required tokens |
|----------|-----------------|
| Telegram | `TELEGRAM_TOKEN` |
| Discord | `DISCORD_TOKEN` |
| Slack | `SLACK_BOT_TOKEN`, `SLACK_APP_TOKEN` |
| Matrix | `MATRIX_HOMESERVER`, `MATRIX_USER`, `MATRIX_PASSWORD` |
| LINE | `LINE_CHANNEL_TOKEN`, `LINE_CHANNEL_SECRET` + `platforms.line.enabled: true` |
| WhatsApp | `WHATSAPP_ACCESS_TOKEN`, `WHATSAPP_PHONE_NUMBER_ID`, `WHATSAPP_VERIFY_TOKEN` + `platforms.whatsapp.enabled: true` |
| Webhook | on by default at `POST /webhook` (port 3001) — configurable via `platforms.webhook` |

**Telegram** — create a bot via [@BotFather](https://t.me/botfather), copy the token.

**Discord** — create an app at [discord.com/developers](https://discord.com/developers/applications), enable **Message Content Intent** under Bot, copy the token.

**Slack** — create an app at [api.slack.com/apps](https://api.slack.com/apps), enable **Socket Mode**, add scopes `chat:write channels:history im:history`, install to workspace.

**Matrix** — works with any homeserver (matrix.org, Synapse, Dendrite, etc.).

**LINE** — create a Messaging API channel at [developers.line.biz](https://developers.line.biz/console/), copy the **Channel access token** and **Channel secret** into `~/.garudust/.env`, then add `platforms.line: { enabled: true, port: 3002, webhook_path: /line }` to `~/.garudust/config.yaml` and point the webhook URL in LINE console to `https://your-host:3002/line`.

**WhatsApp** — create a Meta app at [developers.facebook.com](https://developers.facebook.com/), add the **WhatsApp** product, copy the **Access token** and **Phone number ID** into `~/.garudust/.env`, then add `platforms.whatsapp: { enabled: true, port: 3003, webhook_path: /whatsapp }` to `~/.garudust/config.yaml` and point the webhook URL in Meta console to `https://your-host:3003/whatsapp`. Optionally set `WHATSAPP_APP_SECRET` to enable HMAC signature verification.

---

## LLM Providers

| Provider | `config.yaml` | `.env` (secrets only) | Notes |
|----------|--------------|----------------------|-------|
| Anthropic | `provider: anthropic` | `ANTHROPIC_API_KEY` | Native Messages API; add `base_url` to use a proxy |
| OpenRouter | `provider: openrouter` *(default)* | `OPENROUTER_API_KEY` | 200+ models |
| AWS Bedrock | `provider: bedrock` | `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` | Converse API, SigV4 |
| OpenAI Responses | `provider: codex` | `OPENAI_API_KEY` | `/v1/responses` endpoint |
| Ollama | `provider: ollama` + `base_url` | *(none required)* | Local, no key needed |
| vLLM | `provider: vllm` + `base_url` | `VLLM_API_KEY` | Local OpenAI-compatible server |
| ThaiLLM | `provider: thaillm` | `THAILLM_API_KEY` | NSTDA sovereign Thai LLM |
| Any OpenAI-compat | `provider: custom` + `base_url` | relevant API key | Generic OpenAI-compatible transport |

Set `model`, `provider`, and `base_url` in `config.yaml`. Put only API keys in `~/.garudust/.env`. Switch models at any time with `garudust model`.

---

## Built-in Tools

| Tool | Description |
|------|-------------|
| `web_fetch` | Fetch a URL (static pages) |
| `web_search` | Search the web — uses Serper (Google) when `SERPER_API_KEY` is set, Brave Search when `BRAVE_SEARCH_API_KEY` is set, DuckDuckGo otherwise |
| `http_request` | Make arbitrary HTTP requests (GET/POST/PUT/PATCH/DELETE/HEAD) with custom headers and body; returns status, headers, and body |
| `browser` | Control Chrome/Chromium via CDP — navigate, click, type, screenshot, run JS |
| `read_file` | Read a file from the filesystem |
| `write_file` | Write a file to the filesystem; sensitive credential paths are always blocked |
| `list_directory` | List files and directories; supports glob patterns (`**/*.rs`) and depth limits |
| `terminal` | Run a shell command; sandboxed in Docker when `terminal_sandbox: docker` is set |
| `memory` | Persistent key-value memory (add / read / replace / remove) |
| `user_profile` | Read and update the persistent user profile |
| `session_search` | Full-text search across past conversations (SQLite FTS5) |
| `delegate_task` | Spawn a parallel sub-agent for decomposed work |
| `skills_list` | List available skills |
| `skill_view` | Load a skill's full instructions by name |
| `write_skill` | Create or update a skill in `~/.garudust/skills/` |

**MCP tools** — connect any [MCP](https://modelcontextprotocol.io) server by adding it to the `mcp_servers` list in `config.yaml` (see Configuration).

**Script tools** — add custom tools without writing Rust. Drop a folder containing `tool.yaml` and an optional script into `~/.garudust/tools/` and restart the agent:

```
~/.garudust/tools/
└── get_weather/
    ├── tool.yaml   ← name, description, schema, command
    └── run.py      ← referenced as ./run.py in command (optional)
```

```yaml
# tool.yaml
name: get_weather
description: Get current weather for a city
destructive: false
schema:
  type: object
  properties:
    city:
      type: string
  required: [city]
command: "curl -s wttr.in/{city}?format=3"
```

Parameters are shell-quoted automatically. The command runs with `$TOOL_DIR` set and `current_dir` inside the tool folder, so `./run.py` and sibling files resolve correctly.

### Tool Hub

Install community-built script tools from [garudust-hub](https://github.com/garudust-org/garudust-hub) with one command — no manual folder setup:

```bash
garudust tool list                  # browse available and installed tools
garudust tool install weather       # download to ~/.garudust/tools/weather/
garudust tool install hash_text
garudust tool uninstall weather     # remove tool and its folder
garudust tool update                # re-fetch all hub-installed tools
```

`garudust tool list` shows runtime requirements and descriptions side-by-side:

```
NAME              INSTALLED  AVAILABLE  REQUIRES  DESCRIPTION
----------------------------------------------------------------------
csv_to_json       1.0.0      1.0.0      python3   Convert a CSV file to a JSON array…
fetch_title       —          1.0.0      python3   Fetch the HTML title of a webpage…
hash_text         —          1.0.0      —         Compute the SHA-256 hash of a string
markdown_to_html  —          1.0.0      rust      Convert a Markdown file to HTML…
read_qr           1.0.0      1.0.0      bash      Decode a QR code from an image file
weather           —          1.0.0      bash      Get current weather for a city…
yaml_to_json      —          1.0.0      node      Convert a YAML file to formatted JSON
```

Installed tools are tracked in `~/.garudust/tools/registry.json` and load automatically on every agent start alongside your hand-crafted tools.

| Command | Description |
|---------|-------------|
| `tool list` | Show installed tools and available hub tools side-by-side |
| `tool list --offline` | Show only locally installed tools (no network call) |
| `tool install <name>` | Download from hub into `~/.garudust/tools/<name>/` |
| `tool install <name> --hub <owner/repo>` | Install from a custom hub repository |
| `tool uninstall <name>` | Remove the tool folder and registry entry |
| `tool update` | Re-download all hub tools to the latest version |

To contribute a tool, open a PR at [garudust-org/garudust-hub](https://github.com/garudust-org/garudust-hub).

---

## Architecture

```
  garudust (CLI)              garudust-server
  ┌────────────────────┐    ┌─────────────────────────────────────────────┐
  │  TUI / one-shot    │    │  HTTP /chat · /stream · /ws                 │
  │  setup · config    ├──┐ │  Telegram · Discord · Slack · Matrix · LINE · WhatsApp │
  │  doctor · model    │  │ │  Webhook · Cron                             │
  │  tool · skill      │  │ └──────────────────────────┬──────────────────┘
  └──────────┬─────────┘  │                            │
     install │            └─────────────┬──────────────┘
             │                          ▼
             │                 ┌─────────────────┐
             │                 │      Agent       │
             │                 │   run_loop()     │
             │                 └────────┬─────────┘
             │              ┌───────────┴───────────┐
             │              ▼                       ▼
             │┌──────────────────────┐  ┌─────────────────────────────────┐
             ││      Transport       │  │        ToolRegistry              │
             ││  Anthropic           │  │  web_fetch · web_search          │
             ││  OpenRouter          │  │  http_request · browser          │
             ││  AWS Bedrock         │  │  read_file · write_file          │
             ││  Codex               │  │  list_directory · terminal       │
             ││  Ollama · vLLM       │  │  memory · user_profile           │
             ││  ThaiLLM             │  │  session_search · delegate_task  │
             │└──────────────────────┘  │  script tools · skills           │
             │                          │  MCP (external)                  │
             │                          └─────────────┬───────────────────┘
             │                                        │
             │                            ┌───────────┴───────────┐
             │                            ▼                       ▼
             │                  ┌──────────────────┐  ┌──────────────────────┐
             └─────────────────▶│ FileMemoryStore   │  │      SessionDb       │
                                │ tools/ · skills/  │  │   SQLite + FTS5      │
                                │ memory/           │  └──────────────────────┘
                                └────────┬──────────┘
                                         ▲
                               ┌─────────┴──────────┐
                               │   garudust-hub      │
                               │  community tools    │
                               │  & skills packages  │
                               └────────────────────┘
```

### Crate layout

| Crate / Binary | Role |
|---|---|
| `garudust-core` | Shared traits & types — zero I/O |
| `garudust-transport` | LLM adapters: Anthropic, OpenAI-compat, Bedrock, Codex, Ollama, vLLM, ThaiLLM |
| `garudust-tools` | Tool registry + built-in toolsets (web, files, terminal, browser, …) |
| `garudust-memory` | `FileMemoryStore` (markdown) + `SessionDb` (SQLite + FTS5) |
| `garudust-agent` | Agent run loop, context compressor, prompt builder |
| `garudust-platforms` | Telegram, Discord, Slack, Matrix, LINE, WhatsApp, Webhook |
| `garudust-cron` | Cron scheduler |
| `garudust-gateway` | axum HTTP gateway — `/chat`, `/chat/stream`, `/chat/ws`, `/metrics` |
| `bin/garudust` | CLI: interactive TUI, one-shot, `setup`, `config`, `doctor`, `model` |
| `bin/garudust-server` | Headless: all platforms + HTTP gateway + cron in one process |

---

## Contributing

Garudust is designed to be easy to extend — adding a tool, transport, or platform adapter typically touches exactly one crate and takes under 100 lines.

### Good first issues

- **New tool** — wrap any CLI or API as a `Tool` impl in `garudust-tools`
- **New platform** — implement `PlatformAdapter` (e.g. Signal, WeChat)
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

Have a question or found a bug? Join the [Discord community](https://discord.com/channels/1501414298449088745/1501414298893942877) or open a [GitHub issue](https://github.com/garudust-org/garudust-agent/issues).

---

## License

MIT — see [LICENSE](LICENSE).

---

## Contributors

[![](https://contrib.rocks/image?repo=garudust-org/garudust-agent)](https://github.com/garudust-org/garudust-agent/graphs/contributors)

---

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=garudust-org/garudust-agent&type=Date)](https://star-history.com/#garudust-org/garudust-agent&Date)

---

<div align="center">
  <img src="https://visitor-badge.laobi.icu/badge?page_id=garudust-org.garudust-agent&style=flat" alt="visitors"/>
</div>
