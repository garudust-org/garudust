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
- **Swap providers with one env var** — Anthropic, OpenRouter, AWS Bedrock, Ollama, vLLM, ThaiLLM, or any OpenAI-compatible endpoint
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

```bash
garudust setup   # first-time wizard — pick provider, save API key
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
| `/new` | Start a fresh session |
| `/model <name>` | Switch model on the fly |
| `Ctrl+C` | Quit |

### 2 — One-shot

```bash
garudust "summarise the git log from the last 7 days into a changelog"
```

Output goes to stdout. Exit code is 0 on success. Pipe-friendly.

### 3 — Server

`garudust-server` exposes `POST /chat`, `POST /chat/stream`, and `ws://…/chat/ws`, and runs all platform adapters in the same process. See [Configuration](#configuration) for `.env` and `config.yaml` setup.

**Binary**

```bash
garudust-server --port 3000
```

**Docker** (same binary, containerised)

```bash
# 1. Create a .env file with your secrets (see Configuration below)
cp .env.example .env   # or write it manually
# 2. Start
docker compose up -d
# 3. Check health
curl http://localhost:3000/health
```

Data is persisted in the `garudust-data` Docker volume (`/root/.garudust` inside the container). To use a custom `config.yaml`, bind-mount it:

```yaml
# docker-compose.yml (add to volumes block)
- ./config.yaml:/root/.garudust/config.yaml:ro
```

**API test**

```bash
curl -X POST http://localhost:3000/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "write a haiku about Rust"}'

# Streaming (Server-Sent Events)
curl -X POST http://localhost:3000/chat/stream \
  -H "Content-Type: application/json" \
  -d '{"message": "explain async/await in 3 sentences"}'
```

---

## Configuration

Secrets live in `~/.garudust/.env`. Everything else goes in `~/.garudust/config.yaml`. Run `garudust setup` to generate both interactively.

### `~/.garudust/.env`

```bash
# LLM provider — set at least one
ANTHROPIC_API_KEY=sk-ant-...
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
model: anthropic/claude-sonnet-4-6
provider: anthropic        # anthropic | openrouter | ollama | vllm | bedrock | thaillm

# Platform adapters — set tokens in .env, enable here
platforms:
  telegram:
    enabled: true
  discord:
    enabled: true
  slack:
    enabled: true
  line:
    enabled: true
    port: 3002
    webhook_path: /line        # LINE console webhook → https://your-host:3002/line
  whatsapp:
    enabled: true
    port: 3003
    webhook_path: /whatsapp

security:
  terminal_sandbox: docker     # none | docker — isolate shell commands
  approval_mode: smart         # smart | auto | deny

# Scheduled tasks
cron:
  jobs:
    - schedule: "0 9 * * *"
      task: "Write a morning briefing and save to ~/briefing.md"
  memory_consolidation: "0 3 * * *"   # nightly memory housekeeping
  memory_expiry: "0 4 * * *"          # prune stale memory entries

# Context and performance
context_window: 128000         # adjust for your model
nudge_interval: 5              # memory-save reminder every N turns (0 = off)
```

---

## What's New in v0.4.0

| Feature | Detail |
|---|---|
| Parallel tool execution | Calls grouped by `parallelism_key` — independent tools run concurrently, conflicting writes serialize automatically |
| Credential rotation | `LLM_FALLBACK_API_KEYS=key2,key3` — rotates on auth failure without restarting |
| 3-region compression | Head (original task) + summarized middle + tail (recent turns) always preserved |
| `AgentHooks` trait | `on_turn_start`, `on_session_end`, `on_pre_compress`, `on_delegation` |
| Extended reasoning effort | `Minimal` (512 tokens) → `Low` → `Medium` → `High` → `XHigh` (32k tokens) |
| Sub-agent iteration budget | `sub_agent_max_iterations` caps delegation chains independently of the main agent |
| FTS5 trigram search | Substring session search — `"pythag"` matches `"Pythagorean"`, versioned DB migration included |
| WAL mode fallback | Degrades gracefully on NFS/SMB filesystems instead of crashing |

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
| OpenRouter | `provider: openrouter` *(default)* | `OPENROUTER_API_KEY` |
| AWS Bedrock | `provider: bedrock` | `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` |
| Ollama | `provider: ollama` + `base_url` | *(none)* |
| vLLM | `provider: vllm` + `base_url` | `VLLM_API_KEY` |
| ThaiLLM | `provider: thaillm` | `THAILLM_API_KEY` |
| Any OpenAI-compat | `provider: custom` + `base_url` | relevant key |

Fallback keys: `LLM_FALLBACK_API_KEYS=key2,key3` — rotated automatically on auth failure.

---

## Skills & Memory

The agent saves everything it learns to `~/.garudust/memory/` and loads it at the start of every session. Reusable workflows are automatically written as skills in `~/.garudust/skills/` — no manual prompting needed.

Install skills from the [agentskills.io](https://agentskills.io) hub:

```bash
garudust skill install agentskills-org/hub/git-workflow
garudust tool install weather   # community script tools
```

---

## Contributing

Adding a tool, transport, or platform adapter typically touches one crate and takes under 100 lines. See [CONTRIBUTING.md](CONTRIBUTING.md) for step-by-step guides.

Found a bug or have a question? [Open an issue](https://github.com/garudust-org/garudust-agent/issues) or join the [Discord community](https://discord.com/channels/1501414298449088745/1501414298893942877).

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
