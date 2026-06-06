# Changelog

All notable changes to this project are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning follows [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

## [0.13.7] — 2026-06-06

### Added

- **Native desktop app (`apps/desktop-native`)** — a pure-Rust [egui](https://github.com/emilk/egui)
  GUI with the agent embedded in-process (no webview, WASM, JS, HTTP sidecar, or
  npm). Launches instantly, talks to the agent by direct calls, and reads/writes
  `config.yaml` / `.env` directly. Feature parity with the web dashboard (Chat
  with model picker + Stop, Status, Config with routing editor + key hints,
  masked Secrets with delete) plus Dark/Light theme and 3 font sizes persisted
  across launches. Runs alongside the browser/Tauri dashboard (Leptos) — pick
  native for speed, web for browser access.

### Removed

- **Tauri desktop shell (`apps/desktop`)** — superseded by the native egui app.
  The desktop is now `apps/desktop-native` (native, agent embedded); the browser
  dashboard is still the Leptos SPA served by `garudust-server --features web-ui`.
  Release installers (`.dmg` / `.exe` / `.AppImage` / `.deb`) are now built with
  [`cargo-packager`](https://github.com/crabnebula-dev/cargo-packager) from the
  native binary (no webview/sidecar); download links are unchanged.

### Changed

- **Web/desktop UI rewritten in Rust (Leptos → WASM); JavaScript/TypeScript and
  npm removed.** The dashboard (`web/`) is now a Leptos app built with Trunk
  instead of React/Vite, and the desktop app builds with the Rust `cargo tauri`
  CLI + a shell sidecar-staging script instead of npm. The UI and features are
  unchanged (chat with runtime model picker, Status, Config with routing editor,
  masked Secrets). The only non-Rust artifacts shipped are the auto-generated
  wasm-bindgen glue and the Tailwind CSS. CI builds the SPA with `trunk` and the
  `web` crate is its own cargo workspace (kept out of the core workspace).

## [0.13.6] — 2026-06-05

### Security

- **Desktop sidecar now binds `127.0.0.1`, not `0.0.0.0`** — `garudust-server`
  gained a `--host` flag / `GARUDUST_HOST` env (default `0.0.0.0`, unchanged for
  servers), and the desktop shell passes `--host 127.0.0.1`. Previously the
  desktop app's bundled, auth-less gateway listened on all interfaces, so anyone
  on the same network could reach `/chat` (drive the agent + tools), read masked
  `/api/env`, or change `/api/config` — CORS only blocked browsers, not direct
  clients.

### Added

- **Runtime model switching** — a Model picker above the chat input lists the
  `routing` hints (each `hint · provider/model`) plus the default, sent as the
  per-message `hint`; and a routing editor on the Config page adds/removes those
  hints in the UI. **New chat** (rotate session) and **Stop** (cancel a running
  stream) buttons. The Secrets page can now delete keys (`DELETE /api/env`).
- **Connection state** — the app shows a "Connecting…" splash until the gateway
  is reachable and a "Lost connection" banner if it drops, instead of failing
  each action silently.
- **macOS signing/notarization wiring** — the release workflow passes Apple
  signing secrets to `tauri-action` when present (unsigned otherwise).

## [0.13.5] — 2026-06-05

### Fixed

- **Desktop dev (`tauri dev`) hit the same CORS block** — the Vite dev server
  serves the UI from `http://localhost:5173`, not `tauri://localhost`, so the
  0.13.4 allowlist did not cover it. CORS now also allows `localhost` /
  `127.0.0.1` on any port; arbitrary sites stay denied.
- **Status page went black when no platform adapters run** — `/health` omits
  `checks.platforms` in that case, and the page indexed the missing object and
  crashed the whole window. It now defaults to an empty map, and every page is
  wrapped in an error boundary so a render error shows a message instead of a
  black screen.

### Added

- **Config page: dropdowns and provider-aware models** — Provider, approval mode
  (`auto`/`smart`/`deny`/`interactive`), and terminal sandbox (`none`/`docker`/
  `ssh`) are now selects instead of free text. Choosing a provider fills a
  sensible default model for it (editable; left untouched for self-hosted
  vllm/ollama), and a hint under Model shows which secret the provider needs and
  whether it is already set.

## [0.13.4] — 2026-06-05

### Fixed

- **Desktop app: Config/Secrets pages failed with `TypeError: Load failed`** —
  the Tauri webview (`tauri://localhost`) calls the bundled gateway on
  `http://127.0.0.1:<port>`, a cross-origin request the webview blocked because
  the gateway sent no CORS headers (chat worked because WebSocket is exempt). The
  gateway now returns CORS headers for Tauri webview origins (`tauri://` scheme
  and `tauri.localhost`) only — arbitrary websites still cannot reach a user's
  localhost gateway, and the same-origin web deployment is unaffected.

## [0.13.3] — 2026-06-04

### Added

- **Web dashboard + desktop app** — a React/Vite/Tailwind SPA (`web/`) that talks
  to the existing gateway over its HTTP/WS API, so one UI build runs both in a
  browser and inside a desktop shell. It has a streaming chat pane (over
  `/chat/ws`), plus Status, Config, and Secrets pages. The gateway gained
  `GET`/`PUT /api/config` (read/replace `config.yaml`; saving hot-reloads the
  agent) and `GET`/`PUT /api/env` (list secret keys with constant-width masking;
  write-only set). Secrets never cross the wire: config secret fields are
  `#[serde(skip)]`, env values are masked, and env writes reject line breaks
  (`.env` injection guard). All `/api/*` routes sit behind the existing
  Bearer-token gate. With the optional `web-ui` Cargo feature, the built SPA is
  embedded into `garudust-server` via `rust-embed` and served with SPA fallback —
  preserving the single self-contained binary. A Tauri 2 desktop shell
  (`apps/desktop`) wraps the same SPA and spawns `garudust-server` as a
  loopback-only sidecar (chosen over Electron to keep the app small). The release
  workflow now builds desktop installers — macOS `.dmg`, Windows `.exe`, Linux
  `.AppImage` / `.deb` — and attaches them to each GitHub Release alongside the
  CLI binaries; app icons are committed (generated from `assets/icon.png`).

- **One-line installers** — `scripts/install.sh` (macOS & Linux, all archs incl.
  ARM / Raspberry Pi / WSL) and `scripts/install.ps1` (Windows). Both detect the
  OS/arch, resolve the latest release, verify the SHA-256 checksum, and install
  `garudust` + `garudust-server`; `GARUDUST_VERSION` and `GARUDUST_BIN_DIR`
  override the version and destination. Replaces the previous manual snippet,
  which extracted into a versioned subdirectory and so failed to move the
  binaries onto `PATH`.
- **MCP streamable-HTTP transport** — MCP servers can now be reached over a
  remote streamable-HTTP endpoint, not just local stdio subprocesses. Set
  `url:` on an `mcp_servers` entry to use HTTP (`command`/`args` are ignored when
  `url` is present); omit `url` for the existing stdio behaviour. Lets Garudust
  consume tools from hosted MCP servers across the network.
- **MCP resources & prompts** — Garudust now surfaces an MCP server's
  *resources* and *prompts* primitives, not just its tools. When a connected
  server advertises the capability, four synthetic tools are registered per
  server: `<server>_list_resources` / `<server>_read_resource` and
  `<server>_list_prompts` / `<server>_get_prompt`. Tools that always error are
  never registered — registration is gated on the server's advertised
  capabilities.

### Security

- **Browser tool SSRF guard** — the `browser` tool's `navigate` action now runs
  `net_guard::is_safe_url` before launching Chrome, rejecting non-http(s) schemes
  (`file:`, `chrome:`, `data:`), private/reserved IPs, and cloud metadata
  endpoints. Previously `navigate` called `page.goto` directly, so a
  prompt-injected URL could drive the headless browser into the internal network
  or local filesystem — the one external-fetch tool that bypassed the guard the
  `web` and `webhook` paths already enforce.

---

## [0.13.1] — 2026-05-27

### Security
- **Terminal: `ssh_remote_cwd` shell injection** (HIGH) — `ssh_remote_cwd` is now validated by `validate_remote_cwd()` before being interpolated into the `cd <dir> &&` prefix. Values that are not absolute paths or contain shell metacharacters (`;`, `&`, `|`, `` ` ``, `$`, `>`, `<`, quotes, whitespace, brackets, `*`, `?`, `#`, `^`) are rejected with `ToolError::Execution`. Previously a value like `"/tmp; rm -rf /"` would have been executed on the remote host.
- **WhatsApp: timing attack on `hub.verify_token`** (MEDIUM) — replaced plain `==` string comparison in `handle_verify` with `verify_token_ct()`, a constant-time comparison built on HMAC-SHA256 + `verify_slice`. Prevents an attacker from inferring the expected token length/prefix via response latency.
- **Terminal: fallback API keys not redacted** (MEDIUM) — `collect_secrets()` now includes all keys from `config.fallback_api_keys` (`LLM_FALLBACK_API_KEYS`). Previously these keys could appear unredacted in terminal command output.
- **Agent: `scrub_tag_block` whitespace bypass** (MEDIUM) — `scrub_tag_block` now also strips sloppy-whitespace tag variants such as `< recalled_memory>` and `</ recalled_memory>` that some local/quantised models emit. Previously those variants bypassed scrubbing and could leak injected memory content into model responses.
- **Config: invite code charset/length bypass** (MEDIUM) — `redeem_invite()` now validates codes before hashmap lookup: empty codes, codes longer than 64 characters, and codes containing characters outside `[a-zA-Z0-9_-]` are rejected immediately. Prevents DoS via huge inputs and injection via special characters.
- **Gateway: silent `session_per_user = false`** (MEDIUM) — `GatewayHandler::new()` now emits a `tracing::warn!` when `platform.session_per_user` is `false`, alerting operators that all users share one conversation session.

---

## [0.13.0] — 2026-05-27

### Added
- **Terminal: SSH remote sandbox** — new `terminal_sandbox: ssh` mode routes every terminal tool command through the system `ssh` binary to a configured remote host. Requires `security.ssh_host`; optional fields: `ssh_user`, `ssh_port`, `ssh_key_path`, `ssh_jump_host` (ProxyJump for hosts behind NAT), `ssh_remote_cwd` (prepend `cd <dir> &&` to every command), `ssh_options` (extra `-o` escape hatch). All hardline blocks and the approval gate still apply before the command reaches the remote host. Configurable via `config.yaml` or env vars (`GARUDUST_TERMINAL_SANDBOX=ssh`, `GARUDUST_SSH_HOST`, `GARUDUST_SSH_USER`, `GARUDUST_SSH_PORT`, `GARUDUST_SSH_KEY_PATH`).
- **Dev: pre-push git hook** — `.githooks/pre-push` runs `cargo fmt --all -- --check` then `cargo test --workspace` before every push. Activate with `git config core.hooksPath .githooks`.

### Fixed
- **Terminal: SSH keepalive** — added `ServerAliveInterval=10 ServerAliveCountMax=3` to SSH args so a network drop mid-command is detected in ~30 s rather than silently hanging until the full command timeout fires.

### Security
- **Terminal: SSH hardening** — SSH sandbox uses `BatchMode=yes` (no interactive prompts), `StrictHostKeyChecking=accept-new` (MITM protection), `ConnectTimeout` capped at 30 s, `--` separator before the command (prevents flag injection), and `env_clear()` before spawning `ssh` (secrets never reach the remote host). Caller-supplied `ssh_options` are appended *after* hardened defaults so `BatchMode` and `StrictHostKeyChecking` cannot be overridden.

---

## [0.12.0] — 2026-05-25

### Added
- **TUI: Profile switcher sidebar** — Tab navigation, Space to select profile, runtime switching without restart
- **TUI: Amber-gold border & separators** — improved visual contrast on all terminal themes
- **TUI: Chat scrollbar** — scroll through full conversation history; live skills/tools banner polling every 2 s
- **TUI: Mouse wheel scroll** — scroll chat history with the mouse wheel
- **TUI: Input history recall** — Up/Down arrow recalls previously sent messages; PageUp/PageDown scrolls chat view
- **Platforms: Slack doc attachment ingestion** — files uploaded in Slack are automatically ingested into RAG
- **CLI: `--anthropic-key` / `--api-key` flag** — override the configured provider key at launch time

### Fixed
- **Agent: `required_tools` enforcement** — no longer loops indefinitely when a required tool is not registered in the active schema set
- **Agent: skill-load instruction** — tightened system prompt to require a direct, meaningful match before loading a skill; reduces false-positive triggers on loosely-related prompts
- **TUI: scroll auto-follow** — fixed mouse wheel being unable to escape the `u16::MAX` auto-follow sentinel

### Docs
- README restructured: 3-step Quick Start, OS binary table, full LLM provider table, `.env` example
- Updated `demo-tui.png` screenshot

---

## [0.11.0] — 2026-05-23

### Added
- **Per-user rate limiting** — new `security.rate_limit_rpm_per_user` config option caps requests per (platform, user_id) pair per minute using a fixed-window counter; replies with a Thai message when exceeded (`#139`)
- **Platform health in `/health`** — `/health` now calls `health_check()` on every registered platform adapter; returns `503` and reports each adapter's status under `checks.platforms` when any adapter is degraded
- **Config validation at startup** — `garudust-server` validates the loaded config before accepting requests: non-empty model, routing hints reference known providers, role `approval_mode` values are valid, and no port conflicts between enabled adapters
- **Child tracing spans** — `process_images`, `process_docs`, and `handle_role_command` in `GatewayHandler` are now instrumented with `#[tracing::instrument]` for per-call trace context
- **Per-platform iteration counters** — `Metrics` tracks `garudust_platform_iterations_total{platform="…"}` in addition to the global counter; exposed via `/metrics`
- **Max upload size enforcement** — images and documents exceeding `platform.max_image_bytes` (default 20 MB) or `platform.max_doc_bytes` (default 50 MB) are silently dropped before processing; oversized temp files are cleaned up
- **Image MIME validation** — `process_images` inspects magic bytes (JPEG, PNG, GIF, WebP) before calling `view_image`; files with unrecognised signatures are skipped to prevent passing arbitrary data to the vision tool
- **`read_roles` / `write_roles` helpers** — `GatewayHandler` now uses poison-safe RwLock helpers instead of inline `.unwrap()` calls throughout

### Changed
- **`PlatformAdapter::health_check()`** — new default method on the trait (returns `Ok(())`) so existing adapters don't need changes; override to add real liveness checks
- **`AppState::platform_adapters`** — gateway state now holds a `Vec<Arc<dyn PlatformAdapter>>` so the health handler can iterate them

### Removed
- **`ReasoningEffort::None` variant** — the variant was unreachable: all three transports (Anthropic, OpenAI-compatible, Bedrock) caught it with `_` and returned the same result as `Option::None`. To disable reasoning, leave `reasoning_effort` unset in config

### Fixed
- Several `clippy::pedantic` lints: `write_with_newline`, `unnecessary_map_or`, `map_unwrap_or`, `manual_let_else`, `items_after_statements`, `useless_vec`, `redundant_closure_for_method_calls`

### Infrastructure
- **GitHub Pages** — switched from legacy Jekyll build (which failed with `BadCredentialsError`) to a direct static-file deploy workflow; `docs/` is plain HTML with no Jekyll dependency

---

## [0.10.0] — 2026-05-22

### Added
- **QR code reader pipeline** — images sent to any platform adapter are scanned for QR codes via `read_qr` hub tool; decoded payloads are prepended to the agent's context before vision analysis
- **`/health` platform checks** — health endpoint iterates registered platform adapters and surfaces per-adapter status

---

## [0.9.0] — 2026-05-21

### Added
- **Role-based access control (RBAC)** — `roles:` in `config.yaml` assigns each platform user a role (`admin` / `member` / custom) that controls `approval_mode` and which toolsets/tools they may use. Users without an entry fall back to `default_role` (or the global approver if unset).
- **Bootstrap admin** — when an `admin` role is defined but `roles.users` is empty, the first user to DM the bot is auto-promoted to admin; no manual ID lookup or yaml edit required.
- **`/join` self-registration** — unknown users can request access; admins receive a notification with a pre-built `/role approve <platform:id> <role>` command they can reply with to grant access.
- **Invite codes** — admins issue single-use, time-bounded codes via `/invite <role> [max_uses]`; users redeem with `/join <code>` for instant role assignment.
- **Interactive tool approval over chat** — when a tool call needs approval, the bot DMs the approver and resumes execution on their reply.
- **Runtime role commands** — `/whoami`, `/role list`, `/role add|approve|remove|deny <platform:id> [role]` for live role management without restart.
- **Setup wizard access-control prompt** — `garudust setup` Full mode now offers Open / Invite / Skip; choosing Invite seeds `admin` (auto-approval) and `member` (smart, terminal denied) role definitions automatically.
- **Unit + integration tests for roles** — 24 unit tests for `RolesConfig` / `RolesApprover` and 10 integration tests for the `GatewayHandler` roles flow.

### Changed
- **Auto-assign lowest role to new users** — when `default_role` is set, unknown users are silently granted the lowest-privilege role instead of being blocked at `/join`.
- **Setup wizard preserves provider on reconfigure** — the provider menu now echoes the existing provider name (e.g. `vllm`, `thaillm`, `mistral`) instead of silently falling back to ollama, and preserves the existing base URL for localhost providers so non-default ports survive an Enter-through.
- **Roles flow + TUI redesign** — simplified the roles resolution path and refreshed the TUI status display.

### Removed
- **`platform.allowed_user_ids`** — flat whitelist that silently dropped messages from unlisted users. Superseded by the role system (`roles:`). **Migration:** assign each previously-listed user a role under `roles.users.<platform>` and leave `roles.default_role` unset so unknown users are gated. Existing configs that still contain `allowed_user_ids` will parse (serde ignores unknown fields) but **the whitelist will no longer be enforced** — confirm your `roles:` block is configured before upgrading.

### Documentation
- **Trilingual README updates** — English, Thai, and Chinese READMEs now ship with a Table of Contents, refreshed Architecture diagram, and a full Access Control section covering roles, bootstrap admin, `/join`, `/invite`, and runtime commands.
- **TUI demo screenshot** refreshed in the English README.
- **`config.yaml.example`** — added a commented `roles:` reference block.
- **Quick Start "02 — Configure"** — calls out Full-mode platform adapters and the invite-only access control preset across all three READMEs.
- **Raspberry Pi / Jetson install note** added with a corrected install command.

---

## [0.8.1] — 2026-05-19

### Added
- **Webhook HMAC verification** — `WebhookAdapter` now validates `X-Hub-Signature-256` when `hmac_secret` is configured in `webhook.hmac_secret`; uses constant-time comparison to prevent timing attacks
- **CI test job** — `cargo test --workspace` added to GitHub Actions pipeline between the `check` and `fmt` jobs
- **`TOOL_PARAMS` env injection** — hub tool subprocesses receive the full JSON params as `TOOL_PARAMS` env var, enabling complex multi-field tools without positional-arg fragility
- **Unit tests** — added tests for `agent::scrub_tag_block`, `agent::SessionStore`, `ContextCompressor::should_compress`, and 19 tests in `web.rs` covering SSRF guards, `strip_html_tags`, `percent_decode`, `floor_char_boundary`, and `parse_ddg_html`

### Fixed
- **`AutoApprover` audit log** — every auto-approved tool call now emits a structured `tracing::info!` event with `tool` and `params` fields

---

## [0.8.0] — 2026-05-18

### Added
- **Ralph loop** — agent persists an active goal across turns via `GoalStore` (file-backed, per-session); goal is injected as `<active_goal>` context before every request. Slash commands: `/goal <text>` to set, `/goal` to view, `/cleargoal` to dismiss
- **TTS hub tool** — `tts` in [garudust-hub](https://github.com/garudust-org/garudust-hub) converts text to speech via a pluggable provider profile (iApp Thai TTS pre-configured); returns a WAV file path; raw PCM responses are wrapped in a RIFF header automatically

### Changed
- **Provider profile resolution for hub tools** — `tools.<name>.model` in `config.yaml` now references a named `providers:` profile (e.g. `tts-iapp`) instead of inline `key`/`base_url`; `GARUDUST_BASE_URL` and `GARUDUST_API_KEY` are injected into the tool subprocess from the resolved profile

### Fixed
- `clippy::unused_async` on `resume_pending` — function had no `.await`; removed `async`
- `clippy::collapsible_match` in TUI setup — collapsed nested `if let + inner match` into a single match arm with guard

---

## [0.7.1] — 2026-05-18

### Fixed
- **Streaming one-shot CLI** — `garudust <task>` now streams output token-by-token instead of waiting for the full response; prints `thinking...` while the model works and clears it on first token
- **Tool call display** — tool invocations appear as `▸ tool_name` on stderr during both CLI one-shot and TUI sessions
- **TUI tool-call status bar** — new `ToolCall(String)` event type updates the status line while tools are running
- **Routing hint display** — CLI one-shot prints `▸ routing: <hint> → <model>` before the request when `--hint` is used
- **Skill install message** — `garudust skill install` now shows `✓ Installed skill '<name>' → <path>` instead of a bare confirmation
- **Setup banner** — `garudust setup` now shows an ASCII logo and a colour-coded provider menu
- **Style/lint** — fixed `clippy::map_unwrap_or` and `rustfmt` line-length warnings that caused CI failures

## [0.7.0] — 2026-05-18

### Added
- **Named Provider Profile system** — `providers:` map in `config.yaml` replaces the flat `provider:` / `model:` fields; `providers.default` sets the primary LLM, additional named profiles (e.g. `vision`, `groq-fast`) are used for routing and per-tool overrides
- **11 new LLM provider integrations** — Together AI, Fireworks AI, Cerebras, Perplexity, Cohere, NVIDIA NIM, Alibaba DashScope, ByteDance Doubao, Zhipu AI (GLM), Moonshot (Kimi), Baidu ERNIE — total 24 providers
- **`GARUDUST_FALLBACK_BASE_URL` / `GARUDUST_FALLBACK_API_KEY`** — fallback model now fully resolved through the profile system; script tool subprocesses receive all 6 `GARUDUST_*` env vars (primary + fallback)
- **Post-install env key warning** — `garudust tool install` now checks `env_required` from `tool.yaml` against `~/.garudust/.env` and prints `garudust config set <KEY> <value>` hints for any missing secrets
- **`garudust config set tools.<name>.model`** — per-tool model override is now writable via CLI (previously documented but not implemented)
- **Dynamic secret detection** — `garudust config set` routes any `*_API_KEY`, `*_TOKEN`, `*_SECRET`, `*_PASSWORD` key to `~/.garudust/.env` automatically without needing a hardcoded allowlist
- Architecture section added to all 3 READMEs (EN / TH / ZH) — ASCII flow diagram + subsystem descriptions

### Changed
- Setup wizard rewritten — all providers now configured via the profile system; custom OpenAI-compatible endpoints supported in setup flow
- `providers.default` profile resolution applied in `garudust doctor` and `garudust config show` output
- `config.yaml.example` updated to document all fields including `providers:`, `routing:`, and per-tool model injection env vars

### Fixed
- `fallback_model` was forwarded as a raw string to script subprocesses; it is now resolved through `resolve_to_env_vars`, setting `GARUDUST_FALLBACK_BASE_URL` and `GARUDUST_FALLBACK_API_KEY` correctly
- `garudust config set <DYNAMIC_KEY>` failed for keys not in the hardcoded `SECRET_KEYS` list (e.g. `GOOGLE_AI_API_KEY`); fixed with pattern-based detection
- `garudust config set tools.<name>.model` printed a hint but did not persist the value; fixed with `update_yaml_tool_model`

---

## [0.6.0] — 2026-05-17

### Added
- **Document RAG toolset** — `doc_ingest`, `doc_search`, `doc_list`, `doc_forget` backed by SQLite FTS5 trigram index; supports PDF, TXT, CSV, MD, JSON, DOCX, XLSX
- **Platform document ingestion** — LINE, Discord, Telegram, Slack, WhatsApp adapters receive file attachments, download them to `/tmp`, and ask the user to confirm before indexing; cancellation cleans up temp files automatically
- **Session-scoped RAG isolation** — each chat/group has its own document bucket keyed by `conv_key`; documents from one session are never visible to another
- **`doc_forget`** — remove documents from the RAG index by file name, exact path, or clear all entries for a session
- **Runtime cron management** — `cron_create`, `cron_list`, `cron_delete` tools let agents schedule, inspect, and cancel recurring autonomous tasks via chat without editing `config.yaml`
- **`CronManager` trait** — defined in `garudust-core` to break the potential circular dependency between `garudust-tools` and `garudust-cron`; `CronSlot` type alias exported from `garudust-tools` for cleaner wiring
- Cron scheduler is now always started (even with no static jobs) so runtime tools are available from first boot
- Unit tests for `cron_create`, `cron_list`, `cron_delete` using an in-memory `MockCronManager`

### Fixed
- **RAG wrong bucket** — `run_tool` was passing an empty `conv_key`, causing indexed documents to land in an unnamed bucket and be unfindable by `doc_search`; fixed with `run_tool_scoped` that forwards the correct session key
- **RAG path extraction** — replaced fragile LLM-extracted path from conversation history with a `DashMap<session_key, Vec<DocAttachment>>` that stores the exact download path at receive time
- **Session isolation** — `session_per_user` isolation now scoped to DM chats only; group chats share one session so images sent by any member are visible to all follow-up queries
- **Image history label** — injected immediately into history before `view_image` completes, eliminating a race where follow-up questions arrived before the label appeared

### Changed
- `register_standard_tools` accepts a `cron: Option<CronSlot>` parameter; CLI passes `None`, server passes the live slot
- `build_agent` in `garudust-server` accepts `cron_slot` and threads it through `spawn_config_watcher` for hot-reload compatibility

---

## [0.5.0] — 2026-05-16

### Added
- **6 new provider integrations** — OpenAI, Gemini (native), Groq, Mistral, DeepSeek, and xAI (Grok) added to the transport layer; all discovered via env-key auto-detection
- **Provider routing hints** — `provider_routing` field lets skills and inline annotations steer individual requests to a specific model/provider without changing global config
- **Per-tool and per-skill model override** — `config.yaml` `tools.<name>.model` / `tools.<name>.fallback_model` (and `skills.<name>.*`) run each tool on a dedicated model
- **Hub auto-config on install** — `garudust tool install <tool>` reads `model` / `fallback_model` from the tool's `tool.yaml` and writes them into config automatically; skipped when the user has already set a value (idempotent)
- `GARUDUST_MODEL` / `GARUDUST_FALLBACK_MODEL` env vars forwarded to script-tool subprocesses (e.g. `view_image` reads these instead of hard-coding model names)
- Timestamp label included in gateway image history entries
- Unit tests for agent run-loop tool dispatch, context compressor, and prompt builder
- Unit tests for provider auto-detection and API-key resolution

### Changed
- Default model/provider values deduplicated into shared consts
- Workspace version bumped to `0.5.0`

### Fixed
- Stale doctest in `garudust-agent::run()` signature corrected

---

## [0.4.0] — 2026-05-16

### Added
- **`AgentHooks` trait** — lifecycle callbacks (`on_turn_start`, `on_session_end`, `on_pre_compress`, `on_delegation`) for library embedders
- **Minimal and XHigh reasoning effort levels** — two new `reasoning_effort` variants alongside the existing Low/Medium/High
- **`sub_agent_max_iterations` config** — cap the iteration budget for `delegate_tasks` sub-agents independently of the parent agent
- **FTS5 trigram tokenizer** — SQLite memory/session search now uses FTS5 `trigram` tokenizer for sub-word and partial-string matching
- **`CredentialRotationTransport`** — automatically rotates to a secondary API key when the primary receives a 401/403 auth failure
- **Silent image analysis pipeline** — LINE, Telegram, and Discord adapters extract image attachments and forward them to `view_image` without user intervention
- **Conflict-aware parallel tool execution** (`parallelism_key`) — tools that share a key are serialized; unrelated tools still run concurrently

### Fixed
- **Context compression** now protects the first user turn from being trimmed (prevents empty first-message loops)
- **WAL journal mode** degrades gracefully on NFS/SMB filesystems instead of failing at startup
- **Config watcher** only reloads on content-change filesystem events, not `Access` events (prevents reload loop on Linux/macOS)
- **`view_image` PATH** — `~/.local/bin` prepended so `uv` is found on typical Linux installs
- **Secrets map** — `dotenvy` removed from server; secrets loaded via in-memory map so subprocesses always see current values after hot-reload
- **Provider→env binding** tightened with an explicit tool-env allowlist (prevents env-var leakage across tool calls)
- Pedantic Clippy lints in Telegram and Discord adapters resolved

---

## [0.3.2] — 2026-05-15

### Added
- **Conversation history persistence** — agent saves the sliding-window conversation to `~/.garudust/history/` and restores it on restart (Hermes-style)
- **Per-session history for all platform adapters** — LINE, Telegram, Discord, WhatsApp each maintain independent history files keyed by chat/user ID
- **TUI cursor navigation** — Left/Right/Ctrl+Left/Ctrl+Right/Home/End keys supported in the input box; click-to-position via mouse
- **TUI drag-to-copy** — mouse capture removed so the terminal's native text-selection works again
- `/clear` as an alias for `/new` in TUI and gateway

### Fixed
- TUI double-cursor eliminated (`set_cursor_position` removed; terminal cursor hidden correctly)
- Gateway: `@mention` prefix stripped before routing so `/clear` and task commands work in LINE group chats

---

## [0.3.1] — 2026-05-14

### Added
- **LINE mention detection** — structured mention parsing; bot responds only when explicitly `@mention`ed in group chats; profile lookup scoped per mention
- **`show_usage_footer` config flag** — set `false` to suppress the per-response `[N iter | X tok | ~$cost]` footer
- **`config set` extended** — now accepts `server.*`, `cron.*`, and `platforms.*` key paths (e.g. `garudust config set server.port 9090`)
- `server.port` and all `cron.*` settings moved from env vars to `config.yaml`
- Webhook adapter settings moved to `platforms.*` in `config.yaml`
- `garudust config show` displays the effective resolved `base_url` per provider

### Fixed
- Release CI skips already-published crate versions instead of failing the whole workflow
- Server no longer logs duplicate sandbox-unavailable warnings on config hot-reload
- `VLLM_BASE_URL` and `OLLAMA_BASE_URL` correctly routed to `config.yaml` `base_url` field
- `garudust setup --reconfigure` correctly overwrites existing config values

---

## [0.3.0] — 2026-05-13

### Added
- **`config.yaml` model / provider / base_url** — top-level fields replace env-var-only model configuration; `garudust setup` writes them automatically
- **ThaiLLM provider** — first-class support for the Thai LLM API; auto-detected via `THAILLM_API_KEY`; added to setup wizard
- **`context_window` config** — override the model's advertised context window so the compressor triggers correctly on small-context models (e.g. `context_window: 27168` for Qwen3-14B-AWQ)
- **`disabled_toolsets`** — block entire toolsets from loading to reduce system-prompt token usage
- **`disabled_tools`** — disable individual tools without uninstalling them
- **`required_tools` enforcement** — session frontmatter can declare `required_tools: [name, ...]`; agent re-prompts once if the session ends without calling them
- Context overflow retry — on a 400 "context too large" error the agent automatically reduces `max_tokens` to `ctx/8` and retries

### Fixed
- `<untrusted_memory>` tags echoed back by weak models are now stripped before the next LLM call
- Token estimation uses `bytes / 3` instead of `chars / 4` for correct handling of Thai/CJK scripts
- `web_fetch` response body capped at 50 KB (down from 512 KB) to reduce context flooding
- Safe `max_tokens` computed from actual context limit to prevent 400 overflow on first request
- `usize`→`u32` casts replaced with `try_from` to satisfy Clippy on 32-bit targets
- `required_tools` counter only increments on successful (non-error) tool calls

---

## [0.2.8] — 2026-05-12

### Added
- **Skill source registry** — `~/.garudust/skills/registry.json` tracks whether each skill was installed from the hub (`hub:<repo>`) or self-written by the agent (`local`), mirroring the existing tools registry pattern
- **Skill conflict detection** — installing a hub skill over a locally-written skill logs a warning; using `write_skill` to overwrite a hub skill notifies the agent that a personalized local override was created (restorable with `garudust skill update`)
- **Skill description + source in reflection prompt** — self-improvement reflection now shows `name [hub|local]: description` instead of just names, giving the model enough context to avoid semantically duplicate skills

### Fixed
- **Unbounded memory allocation in streaming** — `tc_acc` now caps tool-call index at 128; a malformed API response with a large index could previously exhaust memory
- **`reasoning_effort` never forwarded** — `AgentConfig.reasoning_effort` was parsed from `config.yaml` but never passed to any transport; now correctly maps to Anthropic `budget_tokens`, OpenAI `reasoning_effort`, and Bedrock `thinkingConfig` (closes #105)
- **Token estimation for non-Latin scripts** — `ContextCompressor::estimate_tokens` used `chars/4`; `String::len()` already returns byte count so behaviour is unchanged, but the intent is now explicit (closes #107)
- **Logo updated** — README (EN/TH/ZH) now uses `logo-agent.jpg`

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

[0.12.0]: https://github.com/garudust-org/garudust-agent/compare/v0.11.0...v0.12.0
[0.11.0]: https://github.com/garudust-org/garudust-agent/compare/v0.10.0...v0.11.0
[0.10.0]: https://github.com/garudust-org/garudust-agent/compare/v0.9.0...v0.10.0
[0.9.0]: https://github.com/garudust-org/garudust-agent/compare/v0.8.1...v0.9.0
[0.8.1]: https://github.com/garudust-org/garudust-agent/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/garudust-org/garudust-agent/compare/v0.7.1...v0.8.0
[0.7.1]: https://github.com/garudust-org/garudust-agent/compare/v0.7.0...v0.7.1
[0.5.0]: https://github.com/garudust-org/garudust-agent/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/garudust-org/garudust-agent/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/garudust-org/garudust-agent/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/garudust-org/garudust-agent/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/garudust-org/garudust-agent/compare/v0.2.8...v0.3.0
[0.2.8]: https://github.com/garudust-org/garudust-agent/compare/v0.2.7...v0.2.8
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
