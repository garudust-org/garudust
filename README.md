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

A self-improving AI agent runtime written in Rust — ~10 MB binary, no runtime dependencies. Chat in the terminal, reply across 7 platforms, or expose a REST + WebSocket API. Connect any MCP server, swap LLM providers with one env var. No telemetry. No lock-in.

<div align="center">
  <img src="assets/demo.svg" alt="Garudust demo"/>
</div>

---

## Quick Start

**01 — Install**

Download a pre-built binary from [GitHub Releases](https://github.com/garudust-org/garudust-agent/releases/latest) (macOS, Linux, Windows, ARM64):

```bash
ARCH=$(uname -m)
[ "$ARCH" = "aarch64" ] && TARGET="aarch64-unknown-linux-musl" || TARGET="x86_64-unknown-linux-musl"
VER=$(curl -s https://api.github.com/repos/garudust-org/garudust-agent/releases/latest | grep tag_name | cut -d'"' -f4)
curl -LO "https://github.com/garudust-org/garudust-agent/releases/latest/download/garudust-${VER}-${TARGET}.tar.gz"
tar -xzf garudust-*.tar.gz && sudo mv garudust garudust-server /usr/local/bin/
```

Or build from source (Rust 1.87+): `git clone https://github.com/garudust-org/garudust-agent && cargo build --release`

---

**02 — Configure**

```bash
garudust setup    # interactive wizard — picks provider, writes config.yaml + .env
```

Or set your key directly in `~/.garudust/.env` (e.g. `ANTHROPIC_API_KEY=sk-ant-...`). See [LLM Providers](#llm-providers) for all supported keys.

---

**03 — Run**

```bash
garudust                           # interactive TUI
garudust "summarise git log"       # one-shot task
garudust --hint fast "check this"  # route to a cheaper model
garudust-server --port 3000        # headless REST + WebSocket server
docker compose up -d
```

<div align="center">
  <img src="assets/demo-tui.png" alt="Garudust TUI" width="700"/>
</div>

| Key | Action |
|-----|--------|
| `Enter` | Send message |
| `↑ ↓` | Scroll history |
| `/new` | New session |
| `/model <name>` | Switch model |
| `Ctrl+C` | Quit |

---

## Why Garudust?

- **~10 MB, < 20 ms cold start** — statically linked, zero runtime deps
- **Self-improving** — learns your preferences, auto-generates reusable skills, corrects itself without being told twice
- **Parallel tool execution** — independent tools run concurrently; conflict-prone calls serialized automatically
- **24 LLM providers** — Anthropic, OpenAI, Gemini, Groq, Ollama, Bedrock, and more — swap with one line in config
- **7 platform adapters** — Telegram, Discord, Slack, Matrix, LINE, WhatsApp, Webhook in one process
- **Secure by design** — Docker sandbox, RBAC, per-user rate limits, automatic secret redaction

---

## Platforms

<div align="center">
  <a href="https://core.telegram.org/bots"><img src="https://img.shields.io/badge/Telegram-2CA5E0?logo=telegram&logoColor=white&style=for-the-badge"/></a>
  <a href="https://discord.com/developers/applications"><img src="https://img.shields.io/badge/Discord-5865F2?logo=discord&logoColor=white&style=for-the-badge"/></a>
  <a href="https://api.slack.com/apps"><img src="https://img.shields.io/badge/Slack-4A154B?logo=slack&logoColor=white&style=for-the-badge"/></a>
  <a href="https://matrix.org"><img src="https://img.shields.io/badge/Matrix-000000?logo=matrix&logoColor=white&style=for-the-badge"/></a>
  <a href="https://developers.line.biz/console/"><img src="https://img.shields.io/badge/LINE-00C300?logo=line&logoColor=white&style=for-the-badge"/></a>
  <a href="https://developers.facebook.com/docs/whatsapp/cloud-api"><img src="https://img.shields.io/badge/WhatsApp-25D366?logo=whatsapp&logoColor=white&style=for-the-badge"/></a>
  <img src="https://img.shields.io/badge/Webhook-6E7681?style=for-the-badge"/>
</div>

All adapters run in the same `garudust-server` process. Set the token in `~/.garudust/.env` and the adapter activates automatically.

---

## LLM Providers

Set `providers.default.name` in `config.yaml` and the corresponding key in `~/.garudust/.env`:

| Provider | `name` | `.env` key |
|----------|--------|------------|
| Anthropic | `anthropic` | `ANTHROPIC_API_KEY` |
| OpenAI | `openai` | `OPENAI_API_KEY` |
| Google Gemini | `gemini` | `GEMINI_API_KEY` |
| Groq | `groq` | `GROQ_API_KEY` |
| Mistral | `mistral` | `MISTRAL_API_KEY` |
| DeepSeek | `deepseek` | `DEEPSEEK_API_KEY` |
| xAI (Grok) | `xai` | `XAI_API_KEY` |
| OpenRouter | `openrouter` | `OPENROUTER_API_KEY` |
| AWS Bedrock | `bedrock` | `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` |
| Ollama | `ollama` | *(none — add `url:` for custom endpoint)* |
| vLLM | `vllm` | `VLLM_API_KEY` |
| ThaiLLM | `thaillm` | `THAILLM_API_KEY` |
| Together AI | `together` | `TOGETHER_API_KEY` |
| Fireworks AI | `fireworks` | `FIREWORKS_API_KEY` |
| Cerebras | `cerebras` | `CEREBRAS_API_KEY` |
| Perplexity | `perplexity` | `PERPLEXITY_API_KEY` |
| Cohere | `cohere` | `COHERE_API_KEY` |
| NVIDIA NIM | `nvidia` | `NVIDIA_API_KEY` |
| Alibaba DashScope | `alibaba` | `DASHSCOPE_API_KEY` |
| ByteDance Doubao | `doubao` | `ARK_API_KEY` |
| Zhipu AI (GLM) | `zhipu` | `ZHIPU_API_KEY` |
| Moonshot (Kimi) | `moonshot` | `MOONSHOT_API_KEY` |
| Baidu ERNIE | `baidu` | `QIANFAN_API_KEY` |
| Any OpenAI-compat | *(omit `name:`, set `url:` in profile)* | relevant key |

Fallback keys: set `LLM_FALLBACK_API_KEYS=key2,key3` in `.env` — rotated automatically on auth failure.

---

## Architecture

```
┌──────────────────────────────────────────────────────────────────────┐
│  bin/garudust (CLI)              bin/garudust-server (Daemon)        │
└────────────────────┬─────────────────────────┬───────────────────────┘
                     │                         │
                     │          ┌──────────────┴───────────────────────┐
                     │          │  garudust-gateway  (server-only)     │
                     │          │  POST /chat · POST /stream · GET /ws │
                     │          │  RBAC · /join · /invite · Metrics    │
                     │          ├──────────────────────────────────────┤
                     │          │  garudust-platforms  (server-only)   │
                     │          │  Telegram · Discord · Slack          │
                     │          │  LINE · Matrix · WhatsApp · Webhook  │
                     │          ├──────────────────────────────────────┤
                     │          │  garudust-cron  (server-only)        │
                     │          │  cron-scheduled autonomous tasks     │
                     │          └──────────────┬───────────────────────┘
                     │                         │
                     ▼                         ▼
┌──────────────────────────────────────────────────────────────────────┐
│                    garudust-agent  (run-loop)                        │
│  load memory → build prompt → LLM call → tool dispatch → repeat     │
└──────┬──────────────┬─────────────────┬─────────────────────────────┘
       ▼              ▼                 ▼
  garudust-      garudust-        garudust-
  transport      tools            memory
  (24 LLMs +    (built-in +      (memory.md +
  key rotation)  hub + MCP)       SQLite + RAG)

garudust-core — shared types · config · traits (used by every crate above)
```

---

## Configuration

Secrets → `~/.garudust/.env`. Everything else → `~/.garudust/config.yaml`.

```yaml
providers:
  default:
    name: anthropic          # see LLM Providers table above for all 24 options
    key: ${ANTHROPIC_API_KEY}
    model: claude-sonnet-4-6

security:
  approval_mode: smart       # auto | smart | deny
  terminal_sandbox: none     # none | docker  ← use docker in production
  rate_limit_rpm: ~          # per-IP limit (~ = unlimited)
  rate_limit_rpm_per_user: ~ # per-(platform, user_id) limit

# Route a single task to a different model without changing the default:
routing:
  fast: groq-fast/llama-3.1-8b-instant
  # then: garudust --hint fast "quick question"
```

For the full config reference (LLM providers, cron, MCP, RBAC, compression, etc.) see [CONTRIBUTING.md](CONTRIBUTING.md).

---

## Tools

Built-in, no configuration needed:

`web_fetch` · `web_search` · `http_request` · `browser` (CDP) · `read_file` · `write_file` · `list_directory` · `terminal` · `memory` · `session_search` · `delegate_task` · `skill_view` · `write_skill` · `doc_ingest` · `doc_search`

**Hub** — community tools and skills from [garudust-hub](https://github.com/garudust-org/garudust-hub):

```bash
garudust tool install hash_text    # script tool → ~/.garudust/tools/hash_text/
garudust tool install read_qr
garudust skill install weather     # Markdown instruction, no subprocess
garudust skill install fetch-title
```

**MCP** — connect any [Model Context Protocol](https://modelcontextprotocol.io) server:

```yaml
mcp_servers:
  - name: filesystem
    command: npx
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
```

**Custom tools** — drop a `tool.yaml` + script in `~/.garudust/tools/<name>/`. Any language. See [garudust-hub](https://github.com/garudust-org/garudust-hub) for examples.

---

## Access Control

Role-based access via `roles:` in `config.yaml`. The first person to DM the bot is auto-promoted to `admin` when no users are assigned yet.

```yaml
roles:
  default_role: member
  definitions:
    admin:  { approval_mode: auto }
    member: { approval_mode: smart, allowed_toolsets: [web, files, memory], denied_tools: [bash] }
    readonly: { approval_mode: deny }
  users:
    telegram:
      "123456789": admin
```

Runtime commands: `/whoami` · `/join [code]` · `/invite <role> [max_uses]` · `/role list|add|approve|remove`

> **Production:** set `terminal_sandbox: docker` to sandbox shell execution, and `max_delegation_depth: 0` to prevent sub-agent chains.

---

## Memory & Skills

The agent saves everything it learns to `~/.garudust/memory/` and loads it at the start of every session — you never need to repeat yourself. Repeating workflows are automatically written as reusable skills in `~/.garudust/skills/` after `auto_skill_threshold` iterations.

---

## Contributing

Garudust is Rust and designed to be extended. Pick your area:

| Area | Where | Effort |
|------|-------|--------|
| Hub tool or skill | [garudust-hub](https://github.com/garudust-org/garudust-hub) — `tool.yaml` + script | Low — no Rust needed |
| Bug reports / docs | [Issues](https://github.com/garudust-org/garudust-agent/issues) | Minimal |
| New LLM provider | `crates/garudust-transport/src/` — impl `ProviderTransport` (2 methods) | Medium |
| New platform adapter | `crates/garudust-platforms/src/` — impl `PlatformAdapter` (2 methods) | Medium |
| Built-in tool | `crates/garudust-tools/src/toolsets/` — impl `Tool`, register in `ToolRegistry::new()` | Medium (~100 lines) |
| Core features | Agent loop, memory, compression, gateway | High |

```bash
git clone https://github.com/garudust-org/garudust-agent
cd garudust-agent
cargo build && cargo test --workspace && cargo clippy --workspace
```

Step-by-step guides for each area: [CONTRIBUTING.md](CONTRIBUTING.md)

**Community:** [Discord](https://discord.com/channels/1501414298449088745/1501414298893942877) · [Issues](https://github.com/garudust-org/garudust-agent/issues) · [Discussions](https://github.com/garudust-org/garudust-agent/discussions) · [dev.to/garudust](https://dev.to/garudust)

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
