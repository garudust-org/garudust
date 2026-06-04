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

A self-improving AI agent runtime written in Rust — ~10 MB binary, no runtime dependencies. Chat in the terminal, reply across 7 platforms, open the web dashboard, run the desktop app, or expose a REST + WebSocket API. Connect any MCP server, swap LLM providers with one env var. No telemetry. No lock-in.

<div align="center">
  <img src="assets/demo.svg" alt="Garudust demo"/>
</div>

---

## Quick Start

**01 — Install**

```bash
curl -fsSL https://raw.githubusercontent.com/garudust-org/garudust-agent/main/scripts/install.sh | sh
```

macOS & Linux, any arch (ARM, Raspberry Pi, WSL). Windows: `irm .../scripts/install.ps1 | iex`. Override with `GARUDUST_VERSION` / `GARUDUST_BIN_DIR`.

<details>
<summary>Manual download or build from source</summary>

Grab a pre-built binary from [GitHub Releases](https://github.com/garudust-org/garudust-agent/releases/latest):

| OS | Architecture | Binary |
|----|-------------|--------|
| macOS | Apple Silicon (M1/M2/M3/M4) | `garudust-*-aarch64-apple-darwin.tar.gz` |
| macOS | Intel | `garudust-*-x86_64-apple-darwin.tar.gz` |
| Linux | x86_64 | `garudust-*-x86_64-unknown-linux-musl.tar.gz` |
| Linux | ARM64 (Raspberry Pi 4/5, Jetson) | `garudust-*-aarch64-unknown-linux-musl.tar.gz` |
| Windows | x86_64 | `garudust-*-x86_64-pc-windows-msvc.zip` |

Or build from source (Rust 1.87+): `git clone https://github.com/garudust-org/garudust-agent && cargo build --release`

</details>

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
garudust-server --port 3000        # headless REST + WebSocket server (+ web dashboard, see below)
docker compose up -d

# Management subcommands
garudust setup                     # interactive first-time setup wizard
garudust doctor                    # check environment and configuration
garudust config show               # view current configuration
garudust config set <key> <value> # set a configuration value
garudust model [<name>]            # get or switch the active model
# Script tools
garudust tool list                 # list installed + available hub tools
garudust tool install <name>       # install a tool from the hub
garudust tool uninstall <name>     # remove an installed tool
garudust tool update [<name>]      # update one tool (omit to update all)

# Skills
garudust skill list                # list installed + available hub skills
garudust skill install <source>    # install from hub / GitHub / URL / well-known
garudust skill uninstall <name>    # remove an installed skill
garudust skill update [<name>]     # update one skill (omit to update all)
garudust skill validate [<path>]   # validate SKILL.md frontmatter
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

## Web dashboard & desktop app

The same React UI — a streaming chat pane plus Status, Config, and Secrets pages.

**Run the web app**

```bash
garudust-server --port 3000      # terminal 1: start the agent server
npm --prefix web install         # terminal 2: first time only
npm --prefix web run dev         # → open http://localhost:5173
```

**Run the desktop app** — a [Tauri](https://tauri.app) window (~10 MB, no bundled
Chromium) that launches the server for you. Setup and installer builds (DMG / EXE /
AppImage / deb) are in [`apps/desktop/README.md`](apps/desktop/README.md):

```bash
npm --prefix apps/desktop run dev   # → opens the desktop window
```

Secrets stay server-side: the Secrets page is masked + write-only, and the desktop
app talks to the server on `127.0.0.1` only.

---

## Features

🪶 **Tiny footprint** — ~10 MB statically linked binary, < 20 ms cold start, zero runtime dependencies. Runs on a Raspberry Pi without Docker.

🧠 **Self-improving** — remembers your preferences and facts across every session. Automatically writes reusable skills after complex multi-step workflows. Cross-session goals stay injected until you mark them done — you never repeat yourself.

🔀 **24 LLM providers, one config line** — Anthropic, OpenAI, Gemini, Groq, Mistral, DeepSeek, Ollama, AWS Bedrock, vLLM, and 15 more. Route tasks to cheaper models with `--hint`, rotate fallback keys automatically on auth failure.

📡 **7 platforms in one process** — Telegram, Discord, Slack, Matrix, LINE, WhatsApp, Webhook. Per-platform RBAC, mention gate, per-user session isolation — each adapter activates the moment its token is in `.env`.

⚡ **Parallel tool execution** — independent tool calls run concurrently; conflict-prone calls serialized by key. 15+ built-in tools: web search, file I/O, browser automation (CDP), terminal, RAG, sub-agent delegation. Connect any MCP server or drop a custom script tool in any language.

🔒 **Secure by design** — three sandbox modes for the terminal tool: direct host, Docker container, or SSH remote host. Hardline blocks on the most destructive commands regardless of sandbox. Approval modes (`auto` / `smart` / `deny`) gate destructive operations. Secrets are redacted from all tool output before the model sees them.

🌐 **Headless API** — `garudust-server` exposes `/chat`, `/stream`, and a WebSocket endpoint — embed in any app or script. Cron-scheduled autonomous tasks run without a user present.

🖥️ **Web dashboard & desktop app** — one React UI (chat + status + config + secrets) served straight from the binary (`web-ui` feature) or wrapped as a tiny Tauri desktop app (DMG / EXE / AppImage / deb). Secrets stay masked and server-side.

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

### `~/.garudust/.env`

```bash
# LLM provider — set one (auto-detected from env when no config.yaml)
ANTHROPIC_API_KEY=sk-ant-...
# OPENAI_API_KEY=sk-...
# GEMINI_API_KEY=AIza...
# GROQ_API_KEY=gsk_...

# Fallback keys — rotated automatically on auth failure
# LLM_FALLBACK_API_KEYS=sk-ant-backup1,sk-ant-backup2

# Platform adapters — set only what you use
TELEGRAM_TOKEN=123456789:AAFxxx
DISCORD_TOKEN=<bot-token>
SLACK_BOT_TOKEN=xoxb-...
SLACK_APP_TOKEN=xapp-...
LINE_CHANNEL_TOKEN=<channel-access-token>
LINE_CHANNEL_SECRET=<32-char-hex>
WHATSAPP_ACCESS_TOKEN=EAAxxxxx
WHATSAPP_PHONE_NUMBER_ID=123456789012345
WHATSAPP_VERIFY_TOKEN=my_verify_token

# Search (optional — falls back to DuckDuckGo)
BRAVE_SEARCH_API_KEY=BSA...
SERPER_API_KEY=...

# Gateway auth
GARUDUST_API_KEY=my-gateway-secret
```

### `~/.garudust/config.yaml`

```yaml
providers:
  default:
    name: anthropic          # see LLM Providers table above for all 24 options
    key: ${ANTHROPIC_API_KEY}
    model: claude-sonnet-4-6

security:
  approval_mode: smart       # auto | smart | deny
  terminal_sandbox: none     # none | docker | ssh
  rate_limit_rpm: ~          # per-IP limit (~ = unlimited)
  rate_limit_rpm_per_user: ~ # per-(platform, user_id) limit

  # ── SSH sandbox (terminal_sandbox: ssh) ──────────────────────────────
  # ssh_host: "192.168.1.50"              # required
  # ssh_user: "pi"                        # optional — defaults to current OS user
  # ssh_port: 22                          # optional — default 22
  # ssh_key_path: ~/.ssh/garudust_pi      # optional — uses ~/.ssh/id_* if unset
  # ssh_jump_host: "bastion.example.com"  # optional — ProxyJump for hosts behind NAT
  # ssh_remote_cwd: "/home/pi/scripts"    # optional — cd here before every command
  # ssh_options: ["IdentitiesOnly=yes"]   # optional — extra -o flags (escape hatch)

# Route a single task to a different model without changing the default:
routing:
  fast: groq-fast/llama-3.1-8b-instant
  # then: garudust --hint fast "quick question"

# Use a cheap model for background skill-reflection (defaults to main model):
reflection_model: groq/llama-3.1-8b-instant

# Conversation window per session — pairs of (user, assistant) turns (default 20):
max_history_pairs: 20
```

For the full config reference (cron, MCP, RBAC, compression, etc.) see [CONTRIBUTING.md](CONTRIBUTING.md).

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

> **Production:** set `terminal_sandbox: docker` (local container) or `terminal_sandbox: ssh` (remote host) to sandbox shell execution, and `max_delegation_depth: 0` to prevent sub-agent chains.

> **Note:** setting `platform.session_per_user: false` causes all users to share one conversation context. The server logs a `WARN` at startup as a reminder. Only safe for single-user deployments.

---

## Terminal Sandbox

The `terminal` tool supports three execution backends:

| Mode | `terminal_sandbox` | Runs on | Requires |
|---|---|---|---|
| Direct host | `none` | Local machine | Nothing |
| Docker container | `docker` | Isolated container | Docker daemon |
| Remote SSH host | `ssh` | Any host with sshd | SSH key auth |

All modes share the same hardline blocks (fork bomb, `rm -rf /`, `mkfs`, etc.) and the same approval gate — the sandbox only controls *where* the command runs.

### SSH Sandbox

Commands are forwarded via the system `ssh` binary to a remote host. Useful for managing a remote server, Raspberry Pi, or build machine without exposing any port to the internet — the agent SSHes *out*, the remote host just needs port 22 open.

**Config fields** (all under `security:` in `config.yaml`):

| Field | Type | Default | Description |
|---|---|---|---|
| `ssh_host` | string | — | **Required.** Remote hostname or IP |
| `ssh_user` | string | current OS user | Login username |
| `ssh_port` | integer | `22` | SSH port |
| `ssh_key_path` | path | `~/.ssh/id_*` | Private key file |
| `ssh_jump_host` | string | — | ProxyJump bastion (`user@host:port`) for hosts behind NAT |
| `ssh_remote_cwd` | string | — | `cd <dir> &&` prepended to every command; must be an absolute path with no shell metacharacters (e.g. `/home/pi/scripts`) |
| `ssh_options` | list | `[]` | Extra `-o key=value` flags (appended after hardened defaults) |

**Environment variable overrides** — no config.yaml required:

```bash
GARUDUST_TERMINAL_SANDBOX=ssh
GARUDUST_SSH_HOST=192.168.1.50
GARUDUST_SSH_USER=pi
GARUDUST_SSH_PORT=22
GARUDUST_SSH_KEY_PATH=/home/user/.ssh/garudust_pi
```

**Minimal working example** — Raspberry Pi behind a home router:

```yaml
security:
  terminal_sandbox: ssh
  ssh_host: "192.168.1.50"
  ssh_user: "pi"
  ssh_key_path: ~/.ssh/garudust_pi
```

**With a bastion** — Pi is reachable only through a public jump server:

```yaml
security:
  terminal_sandbox: ssh
  ssh_host: "pi.internal"
  ssh_user: "pi"
  ssh_key_path: ~/.ssh/garudust_pi
  ssh_jump_host: "bastion.example.com"
```

**Security properties applied automatically:**

- `BatchMode=yes` — no interactive prompts; fails immediately if key auth is rejected
- `StrictHostKeyChecking=accept-new` — auto-trusts first contact, rejects changed host keys (MITM protection)
- `ConnectTimeout` capped at 30 s — no indefinite TCP hangs
- `ServerAliveInterval=10 ServerAliveCountMax=3` — detects dead connections in ~30 s rather than hanging until the command timeout fires
- `--` before the command — prevents a command starting with `-` from being misread as an SSH flag
- `env_clear()` before spawning `ssh` — API keys and secrets never reach the remote host
- `ssh_remote_cwd` is validated as a safe absolute path before use — values containing shell metacharacters (`;` `&` `|` `` ` `` `$` `>` `<` quotes, whitespace, etc.) are rejected at command time, not silently forwarded to the remote shell
- `ssh_options` are appended *after* hardened defaults — `BatchMode` and `StrictHostKeyChecking` cannot be overridden by caller config

---

## Memory & Skills

The agent saves everything it learns to `~/.garudust/memory/` and loads it at the start of every session — you never need to repeat yourself. Repeating workflows are automatically written as reusable skills in `~/.garudust/skills/` after `auto_skill_threshold` iterations. Set `reflection_model` in `config.yaml` to use a cheaper model for this background pass and keep costs down.

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
git config core.hooksPath .githooks   # enable pre-push checks (fmt + tests)
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
