# Garudust

A self-improving AI agent written in Rust — inspired by [Hermes Agent](https://github.com/NousResearch/hermes-agent), rebuilt from the ground up for performance and a minimal footprint.

## Features

- **Multi-provider LLM** — Anthropic direct or any OpenAI-compatible endpoint (OpenRouter, LM Studio, Ollama, …); switch via config, no code changes
- **Tool system** — extensible registry; built-in tools for file I/O, terminal execution, web fetch, web search, memory, and skills
- **Skills** — load `SKILL.md` files from `~/.garudust/skills/`; index injected into every system prompt automatically
- **Persistent memory** — `MEMORY.md` and `USER.md` under `~/.garudust/memories/`, updated mid-session by the agent via the `memory` tool
- **Session persistence** — every run saved to `~/.garudust/state.db` (SQLite WAL + FTS5 full-text search)
- **Context compressor** — automatically summarises old turns before the context window fills up
- **Parallel tool dispatch** — multiple tool calls in one turn run concurrently via `tokio::join_all`
- **Interactive TUI** — `ratatui`-based terminal UI with message history, status bar, and scrolling
- **Messaging platforms** — Telegram, Discord, and HTTP Webhook adapters (feature-gated)
- **Cron scheduler** — schedule recurring agent tasks via `GARUDUST_CRON_JOBS`
- **HTTP API** — `axum` server with `GET /health` and `POST /chat`

---

## Quick Install

### Prerequisites

- Rust 1.75+
- An API key from [OpenRouter](https://openrouter.ai) or [Anthropic](https://console.anthropic.com)

### Build from source

```bash
git clone https://github.com/ninenox/garudust
cd garudust
cargo build --release
```

Add the binaries to your PATH:

```bash
export PATH="$PATH:$(pwd)/target/release"
```

### First-time setup

```bash
garudust setup
```

The wizard picks your provider, stores the API key in `~/.garudust/.env`, and runs `garudust doctor` to confirm everything is wired correctly.

---

## Usage

### Interactive TUI

```bash
garudust
```

`Enter` — send · `↑ ↓` — scroll · `Ctrl+C` — quit

### One-shot task

```bash
garudust "list all Rust files changed in the last 7 days"
```

### Subcommands

| Command | Description |
|---------|-------------|
| `garudust setup` | Interactive first-time wizard |
| `garudust doctor` | Check provider, API key, connectivity, memory dir, session DB |
| `garudust config show` | Show active config and file paths |
| `garudust config set KEY VAL` | Set a config value or secret |

#### `config set` examples

```bash
garudust config set model anthropic/claude-opus-4-7
garudust config set OPENROUTER_API_KEY sk-or-...
garudust config set ANTHROPIC_API_KEY sk-ant-...
```

Secrets (`*_API_KEY`, `*_TOKEN`) go to `~/.garudust/.env`; settings go to `~/.garudust/config.yaml`.

---

## Headless Server

```bash
garudust-server \
  --telegram-token 123456:ABC... \
  --discord-token  Bot_...
```

The server starts all configured platform adapters, the webhook listener, the cron scheduler, and the HTTP gateway simultaneously.

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `OPENROUTER_API_KEY` | — | OpenRouter (or any OpenAI-compat) key |
| `ANTHROPIC_API_KEY` | — | Anthropic key (auto-selects Anthropic transport) |
| `BRAVE_SEARCH_API_KEY` | — | Brave Search API key (enables `web_search` tool) |
| `GARUDUST_MODEL` | `anthropic/claude-sonnet-4-6` | Model identifier |
| `GARUDUST_PORT` | `3000` | HTTP gateway port |
| `GARUDUST_WEBHOOK_PORT` | `3001` | Webhook adapter port (`0` = disabled) |
| `TELEGRAM_TOKEN` | — | Telegram bot token |
| `DISCORD_TOKEN` | — | Discord bot token |
| `GARUDUST_CRON_JOBS` | — | Comma-separated `"cron_expr=task"` pairs |
| `RUST_LOG` | `info` | Log level |

#### Cron example

```bash
GARUDUST_CRON_JOBS="0 9 * * *=Write a morning briefing and save it to ~/briefing.md" \
garudust-server
```

#### Webhook example

```bash
# Server receives POST /webhook and POSTs the agent reply back to callback_url
curl -X POST http://localhost:3001/webhook \
  -H "Content-Type: application/json" \
  -d '{"text":"summarise today","callback_url":"https://your-app/reply"}'
```

#### HTTP chat example

```bash
curl -X POST http://localhost:3000/chat \
  -H "Content-Type: application/json" \
  -d '{"message":"what time is it in Tokyo?"}'
# → {"output":"...","session_id":"...","iterations":1,"input_tokens":120,"output_tokens":30}
```

---

## Built-in Tools

| Tool | Description |
|------|-------------|
| `web_fetch` | Fetch content from a URL |
| `web_search` | Search the web via Brave Search API |
| `read_file` | Read a file from the filesystem |
| `write_file` | Write content to a file |
| `terminal` | Run a shell command (requires approval by default) |
| `memory` | Add / read / replace / remove memory entries |
| `skills_list` | List all available skills |
| `skill_view` | Load a skill's full instructions by name |

---

## Skills

Place a `SKILL.md` anywhere under `~/.garudust/skills/`:

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
platforms: [macos, linux]   # omit to load on all platforms
---

Full instructions here…
```

The agent sees the skills index in every system prompt and loads individual skills with `skill_view`.

---

## Crate Layout

```
crates/
  garudust-core        Traits, types, errors — no I/O; depended on by everything
  garudust-transport   LLM adapters: Anthropic, OpenAI-compatible
  garudust-tools       Tool registry + built-in toolsets
  garudust-memory      FileMemoryStore (markdown) + SessionDb (SQLite + FTS5)
  garudust-agent       Agent run loop, context compressor, prompt builder
  garudust-platforms   Platform adapters: Telegram, Discord, Webhook
  garudust-cron        Cron scheduler — spawns agent.run() on schedule
  garudust-gateway     axum HTTP gateway, SessionRegistry, /health + /chat

bin/
  garudust             CLI binary — TUI, one-shot, setup, doctor, config
  garudust-server      Headless server — platforms + HTTP + cron
```

---

## What's Next

- [ ] Streaming output — SSE in HTTP gateway + delta rendering in TUI
- [ ] `delegate_task` tool — spawn parallel sub-agents for decomposed work
- [ ] `session_search` tool — FTS5 search across past conversations
- [ ] MCP client — connect to external tool servers via `rmcp`
- [ ] Additional transports — Ollama, AWS Bedrock
- [ ] Additional platforms — Slack, Matrix, Signal
- [ ] Browser tool — CDP via `chromiumoxide`
- [ ] `/model`, `/new`, `/memory` slash commands in TUI
- [ ] Docker image + `docker-compose` for one-command server deploy

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT
