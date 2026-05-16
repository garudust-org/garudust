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

A self-improving AI agent runtime written in Rust — delivered as a single ~10 MB binary with no runtime dependencies. Chat in the terminal, reply across 7 platforms, or expose a REST + WebSocket API. Connect any MCP server, let the agent write its own reusable skills, and swap LLM providers with a single env var. No telemetry. No lock-in.

<div align="center">
  <img src="assets/demo.svg" alt="Garudust demo"/>
</div>

---

## Why Garudust?

- **~10 MB binary, < 20 ms cold start** — statically linked, zero runtime dependencies
- **Self-improving** — learns your preferences, auto-generates reusable skills, corrects itself without being told twice
- **Parallel tool execution** — conflict-aware grouping runs independent tools simultaneously, serializes only when necessary
- **Credential rotation** — set `LLM_FALLBACK_API_KEYS` and the agent rotates to the next key on any auth failure, automatically
- **Smart context compression** — 3-region strategy preserves the original task and recent turns; only the middle is summarized
- **Lifecycle hooks** — `AgentHooks` callbacks for every turn, compression event, delegation, and session end
- **agentskills.io compatible** — install community skills from the hub or any GitHub repo with one command
- **7 platform adapters** — Telegram, Discord, Slack, Matrix, LINE, WhatsApp, Webhook — all in one process
- **Swap providers with one env var** — Anthropic, OpenAI, Gemini, Groq, Mistral, DeepSeek, xAI, OpenRouter, AWS Bedrock, Ollama, vLLM, ThaiLLM, or any OpenAI-compatible endpoint
- **Provider routing hints** — map hint names to provider/model pairs in config; pass `--hint fast` to route a single task to a cheaper model without changing the default
- **Per-tool model config** — override the model (and fallback) used by each hub tool or skill script via `tools.<name>.model` in `config.yaml`
- **Secure by design** — Docker sandbox, hardline command blocks, memory-poisoning protection, automatic secret redaction

---

## Install

Download a pre-built binary from [**GitHub Releases**](https://github.com/garudust-org/garudust-agent/releases/latest) — no Rust required:

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

Or build from source (requires Rust 1.87+):

```bash
git clone https://github.com/garudust-org/garudust-agent
cd garudust-agent && cargo build --release
```

---

## Quick Start

### 01 — Install

Download a pre-built binary from [GitHub Releases](https://github.com/garudust-org/garudust-agent/releases/latest):

```bash
curl -LO https://github.com/garudust-org/garudust-agent/releases/latest/download/garudust-linux-x64.tar.gz
tar -xzf garudust-*.tar.gz
sudo mv garudust garudust-server /usr/local/bin/
```

Or build from source (Rust 1.87+):

```bash
git clone https://github.com/garudust-org/garudust-agent
cd garudust-agent && cargo build --release
```

### 02 — Configure

Run the interactive wizard:

```bash
garudust setup   # pick provider → enter API key → choose model
```

Or set directly in `~/.garudust/.env`:

```bash
ANTHROPIC_API_KEY=sk-ant-...
# OPENAI_API_KEY=sk-...
# GROQ_API_KEY=gsk_...
# OPENROUTER_API_KEY=sk-or-...
```

See [Configuration](#configuration) for the full `config.yaml` reference.

### 03 — Run

```bash
# interactive TUI
garudust

# one-shot task
garudust "summarise git log"

# route task to cheaper model
garudust --hint fast "is this correct?"

# headless server (REST + WS)
garudust-server --port 3000

# Docker
docker compose up -d
```

### TUI keyboard shortcuts

<div align="center">
  <img src="assets/demo-tui.png" alt="Garudust TUI" width="800"/>
</div>

| Key | Action |
|-----|--------|
| `Enter` | Send message |
| `↑ ↓` | Scroll history |
| `/new` | Start a fresh session |
| `/model <name>` | Switch model on the fly |
| `Ctrl+C` | Quit |

### Server — API

`garudust-server` exposes `POST /chat`, `POST /chat/stream`, and `ws://…/chat/ws`, and runs all platform adapters in the same process.

```bash
curl -X POST http://localhost:3000/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "write a haiku about Rust"}'

# Streaming (Server-Sent Events)
curl -X POST http://localhost:3000/chat/stream \
  -H "Content-Type: application/json" \
  -d '{"message": "explain async/await in 3 sentences"}'
```

### Server — Docker

```bash
cp .env.example .env        # add your secrets
docker compose up -d
curl http://localhost:3000/health
```

Data persists in the `garudust-data` volume (`/root/.garudust` inside the container). Bind-mount a custom config:

```yaml
# docker-compose.yml — volumes block
- ./config.yaml:/root/.garudust/config.yaml:ro
```

---

## Configuration

Secrets live in `~/.garudust/.env`. Everything else goes in `~/.garudust/config.yaml`. Run `garudust setup` to generate both interactively.

### `~/.garudust/.env`

```bash
# LLM provider — set exactly one (auto-detected from env when no config.yaml)
ANTHROPIC_API_KEY=sk-ant-...
# OPENAI_API_KEY=sk-...
# GEMINI_API_KEY=AIza...
# GROQ_API_KEY=gsk_...
# MISTRAL_API_KEY=...
# DEEPSEEK_API_KEY=sk-...
# XAI_API_KEY=xai-...
# OPENROUTER_API_KEY=sk-or-...
# VLLM_API_KEY=...

# Fallback keys — rotated automatically on auth failure
# LLM_FALLBACK_API_KEYS=sk-ant-backup1,sk-ant-backup2

# Platform adapters — only set what you use
TELEGRAM_TOKEN=123456789:AAFxxx
DISCORD_TOKEN=<bot-token>
SLACK_BOT_TOKEN=xoxb-...
SLACK_APP_TOKEN=xapp-...
LINE_CHANNEL_TOKEN=<channel-access-token>
LINE_CHANNEL_SECRET=<32-char-hex>
WHATSAPP_ACCESS_TOKEN=EAAxxxxx
WHATSAPP_PHONE_NUMBER_ID=123456789012345
WHATSAPP_VERIFY_TOKEN=my_verify_token

# Tools
BRAVE_SEARCH_API_KEY=BSA...      # optional — falls back to DuckDuckGo
SERPER_API_KEY=...               # optional — Google search via Serper

# Gateway security
GARUDUST_API_KEY=my-gateway-secret
```

### `~/.garudust/config.yaml`

```yaml
# ── LLM ─────────────────────────────────────────────────────────────────────
provider: openrouter        # anthropic | openai | gemini | groq | mistral
                            # deepseek | xai | openrouter | ollama | vllm | thaillm | bedrock
model: anthropic/claude-sonnet-4-6
max_iterations: 90
max_output_tokens: 8192
context_window: 128000      # lower for small-context models (e.g. 32768)
reasoning_effort: ~         # low | medium | high  (Claude extended thinking / OpenAI o-series)
show_usage_footer: false

# ── Timeouts & retries ───────────────────────────────────────────────────────
llm_timeout_secs: 120
tool_timeout_secs: 60
llm_max_retries: 3

# ── Provider routing hints (per-task model override) ────────────────────────
# Pass --hint <name> at the CLI, or hint: "name" in the API payload, to swap
# provider/model for that single task without changing the default.
routing:
  fast:   groq/llama-3.1-8b-instant
  vision: openrouter/google/gemini-flash-1.5
  smart:  anthropic/claude-opus-4-7

# ── Per-tool model override ──────────────────────────────────────────────────
# Forwarded as GARUDUST_MODEL / GARUDUST_FALLBACK_MODEL to the tool subprocess.
# Tools that don't read these vars are unaffected (full backward compat).
tools:
  view_image:
    model: openrouter/google/gemini-flash-1.5
    fallback_model: google/gemini-1.5-flash

# ── Disable tools / toolsets ────────────────────────────────────────────────
# disabled_toolsets: [browser, git, notes]
# disabled_tools: [image_read, pdf_read]

# ── Security ─────────────────────────────────────────────────────────────────
security:
  approval_mode: smart        # auto | smart | deny
  terminal_sandbox: none      # none | docker
  rate_limit_rpm: ~           # per-IP limit (~ = unlimited)
  allowed_read_paths: []      # defaults to cwd + home
  allowed_write_paths: []     # defaults to cwd

# ── Memory expiry ────────────────────────────────────────────────────────────
memory_expiry:
  fact_days: 90               # null = never expires
  project_days: 30
  other_days: 60
  preference_days: ~
nudge_interval: 5             # remind save_memory every N tool rounds (0 = off)
auto_skill_threshold: 5       # auto-write skill after N iterations (0 = off)

# ── Platform / group-chat controls ───────────────────────────────────────────
platform:
  require_mention: false      # true = only respond when @mentioned in groups
  bot_username: ""
  session_per_user: true

# ── Webhook platforms ────────────────────────────────────────────────────────
platforms:
  webhook:
    enabled: true
    port: 3001
    webhook_path: /webhook
  line:
    enabled: false
    port: 3002
    webhook_path: /line
  whatsapp:
    enabled: false
    port: 3003
    webhook_path: /whatsapp

# ── HTTP gateway ──────────────────────────────────────────────────────────────
server:
  port: 3000

# ── Cron jobs ────────────────────────────────────────────────────────────────
cron:
  memory_consolidation: "0 3 * * *"   # nightly memory housekeeping
  memory_expiry: "0 4 * * 0"          # weekly expiry sweep
  jobs:
    - schedule: "0 9 * * 1-5"
      task: "Generate a morning briefing and save to ~/briefing.md"

# ── Context compression ───────────────────────────────────────────────────────
compression:
  enabled: true
  threshold_fraction: 0.8     # compress when 80% of context window is used
  model: ~                    # separate model for compression (defaults to main model)

# ── Network ───────────────────────────────────────────────────────────────────
network:
  force_ipv4: false
  proxy: ~                    # http://proxy:8080

# ── MCP servers ───────────────────────────────────────────────────────────────
mcp_servers:
  - name: filesystem
    command: npx
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
```

---

## Platform Adapters

<div align="center">
  <a href="https://core.telegram.org/bots"><img src="https://img.shields.io/badge/Telegram-2CA5E0?logo=telegram&logoColor=white&style=for-the-badge"/></a>
  <a href="https://discord.com/developers/applications"><img src="https://img.shields.io/badge/Discord-5865F2?logo=discord&logoColor=white&style=for-the-badge"/></a>
  <a href="https://api.slack.com/apps"><img src="https://img.shields.io/badge/Slack-4A154B?logo=slack&logoColor=white&style=for-the-badge"/></a>
  <a href="https://matrix.org"><img src="https://img.shields.io/badge/Matrix-000000?logo=matrix&logoColor=white&style=for-the-badge"/></a>
  <a href="https://developers.line.biz/console/"><img src="https://img.shields.io/badge/LINE-00C300?logo=line&logoColor=white&style=for-the-badge"/></a>
  <a href="https://developers.facebook.com/docs/whatsapp/cloud-api"><img src="https://img.shields.io/badge/WhatsApp-25D366?logo=whatsapp&logoColor=white&style=for-the-badge"/></a>
  <img src="https://img.shields.io/badge/Webhook-6E7681?style=for-the-badge"/>
</div>

All adapters run together in the same `garudust-server` process. Set the relevant token in `~/.garudust/.env` and the adapter activates automatically.

---

## LLM Providers

| Provider | `config.yaml` | `.env` |
|----------|--------------|--------|
| Anthropic | `provider: anthropic` | `ANTHROPIC_API_KEY` |
| OpenAI | `provider: openai` | `OPENAI_API_KEY` |
| Google Gemini | `provider: gemini` | `GEMINI_API_KEY` |
| Groq | `provider: groq` | `GROQ_API_KEY` |
| Mistral | `provider: mistral` | `MISTRAL_API_KEY` |
| DeepSeek | `provider: deepseek` | `DEEPSEEK_API_KEY` |
| xAI (Grok) | `provider: xai` | `XAI_API_KEY` |
| OpenRouter | `provider: openrouter` *(default)* | `OPENROUTER_API_KEY` |
| AWS Bedrock | `provider: bedrock` | `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` |
| Ollama | `provider: ollama` + `base_url` | *(none)* |
| vLLM | `provider: vllm` + `base_url` | `VLLM_API_KEY` |
| ThaiLLM | `provider: thaillm` | `THAILLM_API_KEY` |
| Any OpenAI-compat | `provider: custom` + `base_url` | relevant key |

Fallback keys: `LLM_FALLBACK_API_KEYS=key2,key3` — rotated automatically on auth failure.

---

## Tools

Built-in tools are available out of the box — no configuration required.

| Tool | Description |
|------|-------------|
| `web_fetch` | Fetch a URL |
| `web_search` | Search the web (Brave / Serper / DuckDuckGo) |
| `http_request` | Arbitrary HTTP requests with custom headers and body |
| `browser` | Control Chrome/Chromium via CDP — click, type, screenshot, run JS |
| `read_file` / `write_file` | Filesystem read and write |
| `list_directory` | List files with glob patterns and depth limits |
| `terminal` | Run shell commands (Docker sandbox optional) |
| `memory` | Persistent key-value memory across sessions |
| `session_search` | Full-text search across past conversations (FTS5 trigram) |
| `delegate_task` | Spawn a parallel sub-agent for decomposed work |
| `skill_view` / `write_skill` | Load and write reusable skills |

**Custom script tools** — drop a `tool.yaml` + optional script into `~/.garudust/tools/<name>/`:

```yaml
# tool.yaml
name: get_weather
description: Get current weather for a city
schema:
  type: object
  properties:
    city: { type: string }
  required: [city]
command: "curl -s wttr.in/{city}?format=3"
# env_required: [MY_API_KEY]   # forward specific secrets from ~/.garudust/.env
```

Override the model used by a tool in `config.yaml` — the values are forwarded as `GARUDUST_MODEL` / `GARUDUST_FALLBACK_MODEL` env vars to the script:

```yaml
tools:
  get_weather:
    model: groq/llama-3.1-8b-instant
    fallback_model: openrouter/meta-llama/llama-3.1-8b-instruct
```

**MCP** — connect any [Model Context Protocol](https://modelcontextprotocol.io) server in `config.yaml`:

```yaml
mcp_servers:
  - name: filesystem
    command: npx
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
```

---

## Hub

One command to extend the agent with community-built tools and skills.

**Tool Hub** ([garudust-hub](https://github.com/garudust-org/garudust-hub))

```bash
garudust tool list                        # browse available tools
garudust tool install weather             # download to ~/.garudust/tools/weather/
garudust tool install hash_text
garudust tool uninstall weather
garudust tool update                      # re-fetch all hub tools
```

**Skills Hub** ([agentskills.io](https://agentskills.io))

```bash
garudust skill list
garudust skill install agentskills-org/hub/git-workflow
garudust skill install https://example.com/skills/my-skill/SKILL.md
garudust skill uninstall git-workflow
```

---

## Memory

The agent saves everything it learns to `~/.garudust/memory/` and loads it at the start of every session — you never need to repeat yourself. Reusable workflows are automatically written as skills in `~/.garudust/skills/` without any prompting.

```
You: always format JSON with 2-space indent
Agent: Got it — saving to memory.
# Next session: already applied, no reminder needed
```

---

## Contributing

Welcome, garudian! Garudust is built by people who believe AI agents should be fast, private, and user-controlled. Every contribution — a typo fix, a new tool, or a full feature — makes it better for everyone.

### Ways to contribute

| Area | What it involves | Effort |
|------|-----------------|--------|
| Bug reports | Open an issue with steps to reproduce | Minimal |
| Documentation | Fix typos, improve examples, add translations | Low |
| Hub tools | Add a script tool to [garudust-hub](https://github.com/garudust-org/garudust-hub) | Low |
| Skills | Write a reusable skill and share it on [agentskills.io](https://agentskills.io) | Low |
| Platform adapters | Add support for a new chat platform in `garudust-platforms` | Medium |
| Transport providers | Add a new LLM provider in `garudust-transport` | Medium |
| Core features | Agent loop, memory, compression, tools | High |

### Getting started

```bash
git clone https://github.com/garudust-org/garudust-agent
cd garudust-agent
cargo build                   # build all crates
cargo test --workspace        # full test suite
cargo clippy --workspace      # lint check
```

**Adding a built-in tool** — implement `Tool` in `crates/garudust-tools/src/toolsets/`, register it in `ToolRegistry::new()`. Typically one file, under 100 lines.

**Adding a hub tool** — drop a `tool.yaml` + script into [garudust-hub](https://github.com/garudust-org/garudust-hub) under `tools/<name>/`. No Rust required.

**Adding an LLM provider** — implement `ProviderTransport` (`chat` + `chat_stream`) in `crates/garudust-transport/src/`, wire it up in `registry.rs`.

**Adding a platform adapter** — implement `PlatformAdapter` (`send_message` + `start_listening`) in `crates/garudust-platforms/src/`.

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed step-by-step guides on each area.

### Community

- [Discord](https://discord.com/channels/1501414298449088745/1501414298893942877) — chat, questions, and ideas
- [Issues](https://github.com/garudust-org/garudust-agent/issues) — bug reports and feature requests
- [Discussions](https://github.com/garudust-org/garudust-agent/discussions) — longer-form proposals

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
