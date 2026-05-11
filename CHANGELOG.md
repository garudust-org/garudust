# Changelog

All notable changes to this project are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning follows [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

---

## [0.2.7] — 2026-05-11

### Added
- **Serper.dev web search** — `web_search` now uses Serper (Google results) when `SERPER_API_KEY` is set; priority order is Serper → Brave → DuckDuckGo
- **`get_secret()` helper** — reads secrets from real env or `~/.garudust/.env` for Rust tools that don't go through script.rs env forwarding
- **`max_output_tokens` config** — `AgentConfig.max_output_tokens` lets users cap per-request output tokens via `config.yaml` (useful for small-context local models)
- **`serde(default)` on all `AgentConfig` fields** — a minimal `config.yaml` (e.g. only `max_output_tokens: 4096`) now deserialises correctly without silently falling back to defaults
- **Cross-language skill matching** — universal skill-check note now instructs the model to match skills by meaning regardless of the user's language, improving trigger reliability for local models with non-English prompts

### Fixed
- **Panic on multibyte output truncation** — `truncate_output` in `terminal.rs` now uses `floor_char_boundary` instead of raw byte slicing; Thai/CJK/emoji in command output no longer causes a panic
- **UTF-8 corruption in `web_fetch` and `http_request`** — response body truncation at 512 KB now respects char boundaries
- **UTF-8 corruption in `read_file`** — file content truncation at 512 KB now respects char boundaries
- **MSRV compatibility** — replaced `str::floor_char_boundary` (stable 1.91) with an `is_char_boundary` helper to satisfy MSRV 1.87
- **Serper snippet truncation** — snippets now truncated by char boundary to prevent UTF-8 corruption on Thai/CJK results

---

## [0.2.6] — 2026-05-10

### Fixed
- **Script tool env forwarding** — `~/.garudust/.env` variables are now forwarded to script tool subprocesses

---

## [0.2.5] — 2026-05-10

### Added
- **Skill install from hub** — `garudust skill install <name>` downloads skills directly from garudust-hub index
- **TUI startup banner redesign** — Hermes-style banner shows logo, tools by toolset, and installed skills
- **TUI tool and skill count** — sidebar displays tool and skill names at startup

### Fixed
- **Zero-arg tool calls** — transport layer now handles tool calls with no arguments without panicking
- **TUI scroll** — scroll position preserved correctly after streaming output

---

## [0.2.4] — 2026-05-08

### Changed
- crates.io keywords and categories improved for better searchability across all published crates

---

## [0.2.3] — 2026-05-08

### Added
- **Tool Hub** — `garudust tool install/uninstall/update` manages script tools from garudust-hub
- **`REQUIRES` and `DESCRIPTION` columns** in `garudust tool list` output
- **Runtime missing warning** — `garudust tool install` warns when a required runtime (e.g. `python3`, `node`) is not found in PATH
- **Script tool folder layout** — tools now require a `tool.yaml` + optional `scripts/` directory structure

### Fixed
- `runtime_in_path` uses `map_or` instead of `map().unwrap_or()`
- Clippy `similar_names` and `case_sensitive_file_extension_comparisons` in `hub.rs`
- Long iterator chains broken to satisfy `rustfmt`

---

## [0.2.2] — 2026-05-06

### Added
- **WhatsApp Business Cloud API adapter** — full inbound/outbound support via Meta Cloud API; HMAC-SHA256 signature verification, text chunking at 4 096-char boundary, `garudust setup` and `garudust-server` integration
- **`delegate_tasks` tool** — parallel sub-agent execution; spawns multiple sub-agents concurrently via `futures::join_all` and returns all results in original order
- **Per-task token budget** — `max_tokens_per_task: Option<u32>` in `AgentConfig`; stops the agent loop early when the token cap is reached and returns a notice
- **Usage footer** — every completed task now appends `[N iter | Xin Yout tok | ~$cost @ model]` to the output
- **`garudust-core::pricing`** — static per-million-token price table covering Claude 3/4, GPT-4o/mini, and Gemini 1.5/2.x families
- **Token format validation in `garudust setup`** — warns and re-prompts on obviously malformed API keys and platform tokens (Anthropic, OpenRouter, Telegram, Discord, Slack, Matrix, LINE, WhatsApp)

### Fixed
- GitHub issue template URLs updated from old `ninenox/garudust` repo to `garudust-org/garudust-agent`
- Security advisory link in `SECURITY.md` corrected to new repo

### Changed
- `garudust-platforms` crate description updated to include WhatsApp
- All README languages and the landing page updated to list WhatsApp as a supported platform

---

## [0.2.1] — 2026-05-05

### Added
- Crate-level `//!` documentation across all 8 library crates for better [docs.rs](https://docs.rs/garudust-agent) coverage
- TUI demo screenshot (`assets/demo-tui.png`) added to all README languages under the Interactive TUI section
- crates.io version and download badges in all README languages

### Fixed
- `Duration::from_mins()` replaced with `Duration::from_secs(60)` in LINE adapter to satisfy MSRV 1.87
- MSRV declaration corrected from 1.75 to 1.87 (`is_multiple_of` and `LazyLock` require 1.87)
- Doctest examples in `garudust-agent`, `garudust-gateway`, `garudust-memory`, and `garudust-transport` corrected to match actual public API signatures

### Changed
- All 10 crates published to crates.io for the first time
- `.claude/` added to `.gitignore`

---

## [0.2.0] — 2026-05-03

### Added
- **Per-skill tool permissioning** — `SKILL.md` frontmatter can now restrict which tools a skill is allowed to call
- **Graceful shutdown** — `garudust-server` handles `SIGTERM`/`SIGINT` with a configurable drain period before exit
- **LLM and tool timeouts** — per-request timeout configuration to prevent hung runs
- **`pdf_read` tool** — extract text from PDF files
- **`http_request` tool** — generic REST API calls from within an agent session
- **`list_directory` tool** — recursive directory listing with glob support
- **LINE Messaging API adapter** — new platform adapter (`garudust-platforms`)
- **Automated skill-reflection pipeline** — agent writes a reusable skill after complex multi-step tasks
- **JSON Schema validation** — all tool parameters validated against their schema before execution
- **Platform whitelist, mention gate, and per-user session isolation**
- **`/metrics` endpoint protection** — Bearer token gate when `GARUDUST_API_KEY` is set

### Fixed
- DuckDuckGo bot-detection challenge detected and returns an actionable error instead of silently failing
- `doctor` command API-key and connectivity checks are now provider-aware (Anthropic / OpenRouter / Ollama / Bedrock)
- `garudust-server` loads `~/.garudust/.env` before parsing CLI args so env vars are available at startup
- `setup` wizard removes stale provider base-URL vars when switching providers
- `setup` wizard pre-fills existing values and allows skipping fields with Enter
- Terminal tool: read-only git commands bypass the approval gate; shell-operator injection, redirection, and `git diff --no-index` bypass closed
- Memory prefetch: stop-word filter, prefetch cap, and injection hardening
- Sub-agents each receive their own iteration budget
- `read_file` and `web_fetch` output capped at 512 KB to prevent context flooding
- `session_search` reuses `SessionDb` connection instead of opening a new one per call
- Docker healthcheck `curl` missing binary and unexposed webhook/LINE ports fixed

### Changed
- System prompt trimmed by ~50% without behavioral regression
- Architecture diagram and crate layout redesigned in all READMEs

---

## [0.1.1] — 2026-05-02

### Added
- GitHub Pages landing page (`docs/`)
- Docker sandboxed execution via `run_command` — commands run in an isolated container with `--no-new-privileges`
- Prompt injection protection via `<untrusted_memory>` tags in recalled memory
- Structured `INFO`-level audit log for every tool call
- DNS TOCTOU gap closed with a custom `SafeResolver`
- Retry with exponential back-off for transient LLM errors
- Hermes-style constitutional approval replacing `SmartApprover`
- Proactive skill loading — agent injects a universal note and handles multi-language triggers per message
- Hermes-style proactive memory — guidance, prefetch, and recall injection
- Per-category memory expiry with a deterministic cron job
- Brave Search API key prompt in full setup mode
- Improved `garudust setup` wizard and `garudust model` subcommand

### Fixed
- DuckDuckGo fallback search added; untrusted content prompt clarified
- Streamed tool call id/name preserved when sent before arguments
- `register` calls for `WriteSkill`, `DelegateTask`, `UserProfileTool`, and `BrowserTool` in main agent build were missing

---

## [0.1.0] — 2026-04-28

### Added
- Initial public release
- Multi-provider transport layer: Anthropic Claude, Ollama, OpenAI-compatible (OpenRouter / vLLM / LM Studio), AWS Bedrock, Codex
- Platform adapters: Telegram, Discord, Slack, Matrix, LINE, HTTP Webhook
- Built-in toolsets: `files`, `terminal`, `web`, `browser`, `memory`, `skills`, `mcp`, `pdf`, `search`, `delegate`
- Interactive TUI (`garudust`) with real-time streaming
- HTTP gateway (`garudust-server`) with SSE streaming, WebSocket, rate limiting, and session management
- Cron scheduler (`garudust-cron`) for recurring autonomous tasks
- File-based memory store and SQLite session database
- MCP (Model Context Protocol) client support
- Multi-language README (English, Thai, Chinese)
- Pre-built binaries for `x86_64-musl` and `aarch64-apple-darwin` via release workflow
- MIT license

[Unreleased]: https://github.com/garudust-org/garudust-agent/compare/v0.2.7...HEAD
[0.2.7]: https://github.com/garudust-org/garudust-agent/compare/v0.2.6...v0.2.7
[0.2.6]: https://github.com/garudust-org/garudust-agent/compare/v0.2.5...v0.2.6
[0.2.5]: https://github.com/garudust-org/garudust-agent/compare/v0.2.4...v0.2.5
[0.2.4]: https://github.com/garudust-org/garudust-agent/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/garudust-org/garudust-agent/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/garudust-org/garudust-agent/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/garudust-org/garudust-agent/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/garudust-org/garudust-agent/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/garudust-org/garudust-agent/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/garudust-org/garudust-agent/releases/tag/v0.1.0
