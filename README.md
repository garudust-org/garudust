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

### Demo

<div align="center">
  <img src="assets/demo.svg" alt="Garudust demo"/>
</div>

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
- **24 LLM providers, named profiles** — Anthropic, OpenAI, Gemini, Groq, Mistral, DeepSeek, xAI, Together AI, Fireworks, Cerebras, Perplexity, Cohere, NVIDIA NIM, Alibaba DashScope, ByteDance Doubao, Zhipu AI, Moonshot, Baidu ERNIE, OpenRouter, AWS Bedrock, Ollama, vLLM, ThaiLLM, or any OpenAI-compatible endpoint — configure named `providers:` profiles in `config.yaml` and route per-task
- **Provider routing hints** — map hint names to provider/model pairs in config; pass `--hint fast` to route a single task to a cheaper model without changing the default
- **Per-tool model config** — override the model (and fallback) used by each hub tool or skill script via `tools.<name>.model` in `config.yaml`
- **Secure by design** — Docker sandbox, hardline command blocks, memory-poisoning protection, automatic secret redaction

---

## Supported Platforms

Download a pre-built binary from [**GitHub Releases**](https://github.com/garudust-org/garudust-agent/releases/latest) — no Rust required:

| Platform | File |
|----------|------|
| macOS Apple Silicon | `garudust-*-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `garudust-*-x86_64-apple-darwin.tar.gz` |
| Linux x86_64 | `garudust-*-x86_64-unknown-linux-musl.tar.gz` |
| Linux ARM64 | `garudust-*-aarch64-unknown-linux-musl.tar.gz` |
| Windows | `garudust-*-x86_64-pc-windows-msvc.zip` |

---

## Quick Start

**01 — Install**

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

---

**02 — Configure**

Run the interactive wizard — it picks a provider, asks for an API key, and writes `~/.garudust/config.yaml` (with a `providers.default` profile) and `~/.garudust/.env`:

```bash
garudust setup
```

Or drop your key directly into `~/.garudust/.env`. See [Configuration](#configuration) for the full `config.yaml` reference.

---

**03 — Run**

```bash
garudust                                  # interactive TUI
garudust "summarise git log"              # one-shot task
garudust --hint fast "is this correct?"   # route to cheaper model
garudust-server --port 3000               # headless server (REST + WS)
docker compose up -d                      # Docker
```

---

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│  bin/garudust (CLI)            bin/garudust-server (Daemon)      │
│  garudust [task] [--hint H]    garudust-server --port 3000       │
└────────────────────┬───────────────────────────┬─────────────────┘
                     │                           │
                     ▼                           ▼
┌──────────────────────────────────────────────────────────────────┐
│                    garudust-agent  (run-loop)                    │
│                                                                  │
│  1. Load memory.md + user_profile.md                            │
│  2. Build system prompt — inject skills note                    │
│  3. Resolve routing hint → transport + model                    │
│                                                                  │
│  LOOP (max_iterations = 90):                                     │
│    a. LLM call (streaming) → text + tool_calls                  │
│    b. Validate schema → check skill permissions → approval gate  │
│    c. Execute tools (parallel where safe, with timeout)         │
│    d. Wrap untrusted output → append results to history         │
│    e. stop_reason == EndTurn → break                            │
│                                                                  │
│  4. Save conversation → ~/.garudust/conversations/{hash}.json   │
│  5. Persist logs → SessionDb (SQLite)                           │
└──────┬──────────────┬─────────────────┬────────────┬────────────┘
       │              │                 │            │
       ▼              ▼                 ▼            ▼
┌────────────┐ ┌────────────┐ ┌──────────────┐ ┌────────────────┐
│ garudust-  │ │ garudust-  │ │  garudust-   │ │ garudust-      │
│ transport  │ │ tools      │ │  memory      │ │ platforms      │
│            │ │            │ │              │ │                │
│ 24 LLM     │ │ Built-in   │ │ memory.md    │ │ Telegram       │
│ providers  │ │ Hub/Script │ │ user_profile │ │ Discord        │
│ Named      │ │ MCP        │ │ sessions.db  │ │ Slack, Matrix  │
│ profiles   │ │            │ │ docs.db(RAG) │ │ LINE, WhatsApp │
│ Retry +    │ │            │ │              │ │ Webhook        │
│ rotation   │ │            │ │              │ │                │
└────────────┘ └────────────┘ └──────────────┘ └────────────────┘
```

**Transport** — `garudust-transport` resolves `providers.default` (or a named profile) to the right API client: native Anthropic SDK, OpenAI-compatible HTTP, Bedrock, or Ollama. Each client is wrapped with exponential-backoff retry and automatic credential rotation.

**Tools** — three classes: *built-in* (files, terminal, browser, web, memory, git, rag, delegate, cron, notes), *hub/script* (downloaded to `~/.garudust/tools/`, any language), and *MCP* (any Model Context Protocol server). All share the same dispatch path: schema validation → permission check → approval gate → timeout execution.

**Memory** — `FileMemoryStore` writes `memory.md` and `user_profile.md` (Markdown); `SessionDb` persists conversation history and tool-call logs to SQLite; `DocStore` provides FTS5 full-text search for RAG across indexed documents.

**Skills** — Markdown instruction files (`~/.garudust/skills/*.md`) injected as a hint in the system prompt. `skill_view` loads a skill's full body and enforces its declared `required_tools` and `permissions` for that turn. Reusable skills are auto-generated after `auto_skill_threshold` iterations.

**Routing** — `--hint <name>` maps to a `routing:` entry in `config.yaml` (`"profile/model"` or `"provider/model"`), swapping transport and model for a single task without changing the default configuration.

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
# ── Provider profiles ─────────────────────────────────────────────────────────
# providers.default sets the primary LLM. API keys stay in ~/.garudust/.env.
providers:
  default:
    name: anthropic          # anthropic | openai | gemini | groq | mistral | deepseek
                             # xai | openrouter | ollama | vllm | thaillm | bedrock
                             # together | fireworks | cerebras | perplexity | cohere
                             # nvidia | alibaba | doubao | zhipu | moonshot | baidu
    key: ${ANTHROPIC_API_KEY}
    model: claude-sonnet-4-6

  # Additional named profiles for routing or per-tool model overrides:
  # groq-fast:
  #   name: groq
  #   key: ${GROQ_API_KEY}
  #   model: llama-3.1-8b-instant
  #
  # local:
  #   url: http://localhost:11434/v1   # custom OpenAI-compatible endpoint
  #   model: llama3.2
  #
  # vision:                            # primary model for view_image tool
  #   name: google
  #   key: ${GOOGLE_AI_API_KEY}
  #   model: google/gemini-2.5-pro
  #
  # vision-fallback:                   # fallback when vision quota/rate-limit hit
  #   name: openrouter
  #   key: ${OPENROUTER_API_KEY}
  #   model: nvidia/nemotron-nano-12b-v2-vl:free

# ── Agent settings ────────────────────────────────────────────────────────────
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
# Pass --hint <name> at the CLI, or hint: "name" in the API payload.
# Format: "profile/model" (uses a named profile) or "provider/model" (builtin).
routing:
  fast:   groq-fast/llama-3.1-8b-instant   # uses the groq-fast profile above
  vision: openai/gpt-4o                     # builtin provider name
  smart:  anthropic/claude-opus-4-7

# ── Per-tool model override ──────────────────────────────────────────────────
# Each slot value is a named entry from providers: above.
# Slot names containing "fallback" inject GARUDUST_FALLBACK_* env vars;
# all others inject GARUDUST_* (primary).
# tools:
#   view_image:
#     model: vision              # references providers.vision profile (define above)
#     model-fallback: vision-fallback

# ── Disable tools / toolsets ────────────────────────────────────────────────
# disabled_toolsets: [browser, git, notes]
# disabled_tools: [image_read, pdf_read]

# ── Security ─────────────────────────────────────────────────────────────────
security:
  approval_mode: smart        # auto | smart | deny
                              # smart = audits risky calls but does NOT block them;
                              # use deny to block all tool use without explicit allow-list
  terminal_sandbox: none      # none | docker
                              # WARNING: none runs shell commands directly on the host.
                              # Use docker in production to isolate command execution.
  rate_limit_rpm: ~           # per-IP limit (~ = unlimited)
  allowed_read_paths: []      # defaults to cwd + home
  allowed_write_paths: []     # defaults to cwd

# ── Sub-agent delegation ──────────────────────────────────────────────────────
# max_delegation_depth: 1     # max recursive depth of delegate_task (default 1)
                              # depth 0 = sub-agents cannot delegate further
                              # depth 1 = sub-agents may spawn one more level (default)
                              # Prevents runaway recursive delegation chains.

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

Set `providers.default.name` in `config.yaml` and the corresponding key in `~/.garudust/.env`:

| Provider | `providers.default.name` | `.env` |
|----------|--------------------------|--------|
| Anthropic | `anthropic` | `ANTHROPIC_API_KEY` |
| OpenAI | `openai` | `OPENAI_API_KEY` |
| Google Gemini | `gemini` | `GEMINI_API_KEY` |
| Groq | `groq` | `GROQ_API_KEY` |
| Mistral | `mistral` | `MISTRAL_API_KEY` |
| DeepSeek | `deepseek` | `DEEPSEEK_API_KEY` |
| xAI (Grok) | `xai` | `XAI_API_KEY` |
| OpenRouter | `openrouter` | `OPENROUTER_API_KEY` |
| AWS Bedrock | `bedrock` | `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` |
| Ollama | `ollama` *(add `url:` for custom endpoint)* | *(none)* |
| vLLM | `vllm` *(add `url:` for custom endpoint)* | `VLLM_API_KEY` |
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
| `terminal` | Run shell commands (Docker sandbox optional — see security note below) |
| `memory` | Persistent key-value memory across sessions |
| `session_search` | Full-text search across past conversations (FTS5 trigram) |
| `delegate_task` | Spawn a parallel sub-agent for decomposed work (depth-limited by `max_delegation_depth`) |
| `skill_view` / `write_skill` | Load and write reusable skills |
| `doc_ingest` | Index a document (PDF, TXT, CSV, MD, …) for full-text search |
| `doc_search` | Full-text search across all indexed documents |
| `doc_list` | List all documents indexed in the current session |
| `doc_forget` | Remove one or all documents from the RAG index |

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

Override the model used by a tool in `config.yaml`. Slot values are named entries from `providers:`. Slots whose name contains `"fallback"` inject `GARUDUST_FALLBACK_*` env vars; all others inject `GARUDUST_*` (primary). The subprocess receives `GARUDUST_MODEL` / `GARUDUST_BASE_URL` / `GARUDUST_API_KEY` and `GARUDUST_FALLBACK_MODEL` / `GARUDUST_FALLBACK_BASE_URL` / `GARUDUST_FALLBACK_API_KEY`:

```yaml
providers:
  vision:
    name: google
    key: ${GOOGLE_AI_API_KEY}
    model: google/gemini-2.5-pro
  vision-fallback:
    name: openrouter
    key: ${OPENROUTER_API_KEY}
    model: nvidia/nemotron-nano-12b-v2-vl:free

tools:
  view_image:
    model: vision              # references providers.vision profile
    model-fallback: vision-fallback
```

**MCP** — connect any [Model Context Protocol](https://modelcontextprotocol.io) server in `config.yaml`:

```yaml
mcp_servers:
  - name: filesystem
    command: npx
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
```

---

## RAG (Document Search)

Send a document to the bot and ask questions about it. The agent indexes the file locally and searches it automatically when you ask a relevant question.

**Supported formats:** PDF, TXT, CSV, MD, JSON, DOCX, DOC, XLSX, XLS

### Via chat platforms (LINE, Telegram, Discord, …)

Send a document file → the bot asks for confirmation in your language → reply to confirm → the file is indexed and searchable.

```
[You send price_list.pdf]
Bot: You sent "price_list.pdf". Would you like me to index it so I can answer questions about it?
You: Yes
Bot: Indexed 12 chunks from price_list.pdf.
You: What is the price of item B?
Bot: According to price_list.pdf, item B costs 250 baht.
```

### Via CLI or agent

Mention the file path and the agent will ingest it:

```
You: Read and index /home/user/report.pdf so I can ask questions about it.
```

### Searching

Ask questions naturally — the agent calls `doc_search` automatically:

```
You: What was the total revenue in Q3?
You: Summarise the key points from the uploaded document.
```

### Listing indexed documents

```
You: Which documents have been indexed?
Agent: [calls doc_list — returns file names, chunk counts, and ingest time]
```

### Removing documents

Remove by file name, exact path, or clear everything:

```
You: Forget price_list.pdf
You: Remove all indexed documents
```

### Data isolation

Each group chat, DM, and platform session has its own isolated document index — documents indexed in one chat are never visible in another.

### Disabling RAG

RAG is enabled by default. To disable it:

```yaml
disabled_toolsets: ["rag"]
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

## Security Notes

### Terminal tool

`terminal_sandbox: none` (the default) executes shell commands **directly on the host OS**. Any command the agent chooses to run will have the same permissions as the server process.

- **For development / local CLI use:** the default is acceptable.
- **For production / multi-user deployments:** set `terminal_sandbox: docker` to isolate command execution in a Docker container, or disable the terminal tool entirely:

```yaml
security:
  terminal_sandbox: docker   # recommended for production

# or disable the tool entirely:
disabled_tools: [terminal]
```

`approval_mode: smart` audits potentially risky calls and logs them, but does **not** block execution. To require explicit approval or deny all unapproved tool use, change the mode:

```yaml
security:
  approval_mode: deny        # block all tool use not in an allow-list
```

### delegate_task recursion

`delegate_task` spawns a sub-agent. Without a depth limit, a malicious or misconfigured prompt could trigger unbounded recursive delegation. The default `max_delegation_depth: 1` means a sub-agent can spawn one further level of sub-agents, but no deeper. Set to `0` to prevent sub-agents from delegating at all:

```yaml
max_delegation_depth: 0   # sub-agents cannot spawn further sub-agents
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
- [dev.to/garudust](https://dev.to/garudust) — articles and tutorials

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
