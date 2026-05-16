<div align="center">
  <img src="../../../assets/logo-agent.jpg" alt="Garudust"/>

  <a href="../../../README.md"><img src="https://img.shields.io/badge/🇺🇸-English-blue?style=flat-square" alt="English"/></a>
  <a href="../th/README.md"><img src="https://img.shields.io/badge/🇹🇭-ภาษาไทย-red?style=flat-square" alt="ภาษาไทย"/></a>
  <a href="../zh/README.md"><img src="https://img.shields.io/badge/🇨🇳-简体中文-yellow?style=flat-square" alt="简体中文"/></a>
</div>

# Garudust Agent

[![CI](https://github.com/garudust-org/garudust-agent/actions/workflows/ci.yml/badge.svg)](https://github.com/garudust-org/garudust-agent/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/garudust-agent.svg)](https://crates.io/crates/garudust-agent)
[![Downloads](https://img.shields.io/crates/d/garudust-agent.svg)](https://crates.io/crates/garudust-agent)
[![Release](https://img.shields.io/github/v/release/garudust-org/garudust-agent)](https://github.com/garudust-org/garudust-agent/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](../../../LICENSE)
![Rust 1.87+](https://img.shields.io/badge/rust-1.87+-orange.svg)
[![Discord](https://img.shields.io/badge/Discord-加入社区-5865F2?logo=discord&logoColor=white&style=flat-square)](https://discord.com/channels/1501414298449088745/1501414298893942877)

**你的 AI 智能体。你的服务器。你的规则。**

基于 Rust 构建的自进化 AI 智能体运行时 — 单一 ~10 MB binary，零运行时依赖。终端聊天、跨 7 大平台回复、开放 REST + WebSocket API，三者兼得。连接任意 MCP 服务器，让智能体自动编写可复用技能，一个环境变量即可切换 LLM 提供商。无遥测，无供应商锁定。

<div align="center">
  <img src="../../../assets/demo.svg" alt="Garudust demo"/>
</div>

---

## 为什么选择 Garudust？

- **~10 MB binary，冷启动 < 20 ms** — 单一静态链接文件，无任何运行时依赖
- **自我进化** — 记住你的偏好，将可复用工作流自动保存为技能，无需重复提醒
- **并行工具执行** — 基于 `parallelism_key` 分组，独立工具并发运行，仅在必要时串行化
- **自动凭证轮换** — 设置 `LLM_FALLBACK_API_KEYS`，遇到鉴权失败时自动切换下一个密钥，无需重启
- **智能上下文压缩** — 三区域策略：保留原始任务与最新轮次，仅压缩中间部分
- **生命周期钩子** — `AgentHooks` 回调覆盖每轮对话、压缩事件、委派及会话结束
- **兼容 agentskills.io** — 一条命令从社区 hub 或任意 GitHub 仓库安装技能
- **7 大平台适配器** — Telegram、Discord、Slack、Matrix、LINE、WhatsApp、Webhook，同进程运行
- **一个环境变量切换提供商** — 支持 Anthropic、OpenAI、Gemini、Groq、Mistral、DeepSeek、xAI、OpenRouter、AWS Bedrock、Ollama、vLLM、ThaiLLM
- **提供商路由 hint** — 在 config 中将 hint 名称映射到 provider/model 对；传入 `--hint fast` 即可仅针对该任务切换到更廉价的模型，不影响默认配置
- **按工具配置模型** — 通过 `config.yaml` 中的 `tools.<name>.model` 为每个 hub 工具或技能脚本指定模型（及备用模型）
- **安全优先设计** — Docker 沙箱、硬性命令拦截、内存投毒防护、工具输出自动脱敏

---

## 安装

从 [**GitHub Releases**](https://github.com/garudust-org/garudust-agent/releases/latest) 下载预构建 binary — 无需安装 Rust：

| 平台 | 文件 |
|------|------|
| macOS Apple Silicon | `garudust-*-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `garudust-*-x86_64-apple-darwin.tar.gz` |
| Linux x86_64 | `garudust-*-x86_64-unknown-linux-musl.tar.gz` |
| Linux ARM64 | `garudust-*-aarch64-unknown-linux-musl.tar.gz` |
| Windows | `garudust-*-x86_64-pc-windows-msvc.zip` |

```bash
tar -xzf garudust-*.tar.gz
sudo mv garudust garudust-server /usr/local/bin/
```

或从源码构建（需 Rust 1.87+）：

```bash
git clone https://github.com/garudust-org/garudust-agent
cd garudust-agent && cargo build --release
```

---

## 快速开始

<table>
<tr><td>
<h3>01 — 安装</h3>
从 <a href="https://github.com/garudust-org/garudust-agent/releases/latest">GitHub Releases</a> 下载预构建 binary：
<pre>curl -LO https://github.com/garudust-org/garudust-agent/releases/latest/download/garudust-linux-x64.tar.gz
tar -xzf garudust-*.tar.gz
sudo mv garudust garudust-server /usr/local/bin/</pre>
或从源码构建（需 Rust 1.87+）：
<pre>git clone https://github.com/garudust-org/garudust-agent
cd garudust-agent && cargo build --release</pre>
</td></tr>
<tr><td>
<h3>02 — 配置</h3>
运行首次配置向导：
<pre>garudust setup   # 选择提供商 → 输入 API 密钥 → 选择模型</pre>
或直接写入 <code>~/.garudust/.env</code>：
<pre>ANTHROPIC_API_KEY=sk-ant-...
# OPENAI_API_KEY=sk-...
# GROQ_API_KEY=gsk_...
# OPENROUTER_API_KEY=sk-or-...</pre>
完整 <code>config.yaml</code> 参考见<a href="#配置">配置</a>章节。
</td></tr>
<tr><td>
<h3>03 — 运行</h3>
<pre># 交互式 TUI
garudust

# 单次任务
garudust "整理 git log 为 changelog"

# 使用更廉价的模型
garudust --hint fast "这段代码正确吗？"

# 无头服务器（REST + WS）
garudust-server --port 3000

# Docker
docker compose up -d</pre>
</td></tr>
</table>

### TUI 快捷键

<div align="center">
  <img src="../../../assets/demo-tui.png" alt="Garudust TUI" width="800"/>
</div>

| 按键 | 操作 |
|------|------|
| `Enter` | 发送消息 |
| `↑ ↓` | 滚动历史 |
| `/new` | 开始新会话 |
| `/model <name>` | 即时切换模型 |
| `Ctrl+C` | 退出 |

### 服务器 — API

`garudust-server` 开放 `POST /chat`、`POST /chat/stream` 和 `ws://…/chat/ws`，并在同一进程中运行所有平台适配器。

```bash
curl -X POST http://localhost:3000/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "写一首关于 Rust 的俳句"}'

# 流式输出（Server-Sent Events）
curl -X POST http://localhost:3000/chat/stream \
  -H "Content-Type: application/json" \
  -d '{"message": "用 3 句话解释 async/await"}'
```

### 服务器 — Docker

```bash
cp .env.example .env        # 填入你的密钥
docker compose up -d
curl http://localhost:3000/health
```

数据持久化在 volume `garudust-data`（容器内为 `/root/.garudust`），可 bind-mount 自定义配置：

```yaml
# docker-compose.yml — volumes 块
- ./config.yaml:/root/.garudust/config.yaml:ro
```

---

## 配置

密钥存放在 `~/.garudust/.env`，其余配置放在 `~/.garudust/config.yaml`。运行 `garudust setup` 可交互式生成两个文件。

### `~/.garudust/.env`

```bash
# LLM 提供商 — 设置一个（无 config.yaml 时自动从环境变量检测）
ANTHROPIC_API_KEY=sk-ant-...
# OPENAI_API_KEY=sk-...
# GEMINI_API_KEY=AIza...
# GROQ_API_KEY=gsk_...
# MISTRAL_API_KEY=...
# DEEPSEEK_API_KEY=sk-...
# XAI_API_KEY=xai-...
# OPENROUTER_API_KEY=sk-or-...
# VLLM_API_KEY=...

# 备用密钥 — 鉴权失败时自动轮换
# LLM_FALLBACK_API_KEYS=sk-ant-backup1,sk-ant-backup2

# 平台适配器 — 仅设置需要使用的
TELEGRAM_TOKEN=123456789:AAFxxx
DISCORD_TOKEN=<bot-token>
SLACK_BOT_TOKEN=xoxb-...
SLACK_APP_TOKEN=xapp-...
LINE_CHANNEL_TOKEN=<channel-access-token>
LINE_CHANNEL_SECRET=<32位十六进制密钥>
WHATSAPP_ACCESS_TOKEN=EAAxxxxx
WHATSAPP_PHONE_NUMBER_ID=123456789012345
WHATSAPP_VERIFY_TOKEN=my_verify_token

# 搜索工具
BRAVE_SEARCH_API_KEY=BSA...      # 可选 — 未设置时回退到 DuckDuckGo
SERPER_API_KEY=...               # 可选 — 通过 Serper 使用 Google 搜索

# 网关安全
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
context_window: 128000      # 小上下文模型请调低（如 32768）
reasoning_effort: ~         # low | medium | high（Claude 扩展思考 / OpenAI o-series）
show_usage_footer: false

# ── 超时与重试 ────────────────────────────────────────────────────────────────
llm_timeout_secs: 120
tool_timeout_secs: 60
llm_max_retries: 3

# ── 提供商路由 hint（按任务切换模型）────────────────────────────────────────
# 在 CLI 传入 --hint <name>，或在 API payload 中设置 hint: "name"，
# 仅针对该任务切换 provider/model，不影响默认配置。
routing:
  fast:   groq/llama-3.1-8b-instant
  vision: openrouter/google/gemini-flash-1.5
  smart:  anthropic/claude-opus-4-7

# ── 按工具配置模型 ────────────────────────────────────────────────────────────
# 以 GARUDUST_MODEL / GARUDUST_FALLBACK_MODEL 环境变量形式传递给工具子进程。
# 不读取这些变量的工具不受影响（完全向后兼容）。
tools:
  view_image:
    model: openrouter/google/gemini-flash-1.5
    fallback_model: google/gemini-1.5-flash

# ── 禁用工具 / 工具集 ─────────────────────────────────────────────────────────
# disabled_toolsets: [browser, git, notes]
# disabled_tools: [image_read, pdf_read]

# ── 安全 ──────────────────────────────────────────────────────────────────────
security:
  approval_mode: smart        # auto | smart | deny
  terminal_sandbox: none      # none | docker
  rate_limit_rpm: ~           # 每 IP 每分钟请求限制（~ = 不限）
  allowed_read_paths: []      # 默认：cwd + home
  allowed_write_paths: []     # 默认：cwd

# ── 记忆过期 ──────────────────────────────────────────────────────────────────
memory_expiry:
  fact_days: 90               # null = 永不过期
  project_days: 30
  other_days: 60
  preference_days: ~
nudge_interval: 5             # 每 N 轮工具调用后提醒保存记忆（0 = 关闭）
auto_skill_threshold: 5       # 达到 N 次迭代后自动写入技能（0 = 关闭）

# ── 平台 / 群聊控制 ───────────────────────────────────────────────────────────
platform:
  require_mention: false      # true = 仅在群组中被 @提及时才响应
  bot_username: ""
  session_per_user: true

# ── Webhook 平台 ──────────────────────────────────────────────────────────────
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

# ── HTTP 网关 ─────────────────────────────────────────────────────────────────
server:
  port: 3000

# ── 定时任务 ──────────────────────────────────────────────────────────────────
cron:
  memory_consolidation: "0 3 * * *"   # 每晚自动整理记忆
  memory_expiry: "0 4 * * 0"          # 每周清理过期记忆
  jobs:
    - schedule: "0 9 * * 1-5"
      task: "生成每日简报并保存到 ~/briefing.md"

# ── 上下文压缩 ────────────────────────────────────────────────────────────────
compression:
  enabled: true
  threshold_fraction: 0.8     # 上下文窗口使用达 80% 时触发压缩
  model: ~                    # 独立压缩模型（默认使用主模型）

# ── 网络 ──────────────────────────────────────────────────────────────────────
network:
  force_ipv4: false
  proxy: ~                    # http://proxy:8080

# ── MCP 服务器 ────────────────────────────────────────────────────────────────
mcp_servers:
  - name: filesystem
    command: npx
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
```

---

## 平台适配器

<div align="center">
  <a href="https://core.telegram.org/bots"><img src="https://img.shields.io/badge/Telegram-2CA5E0?logo=telegram&logoColor=white&style=for-the-badge"/></a>
  <a href="https://discord.com/developers/applications"><img src="https://img.shields.io/badge/Discord-5865F2?logo=discord&logoColor=white&style=for-the-badge"/></a>
  <a href="https://api.slack.com/apps"><img src="https://img.shields.io/badge/Slack-4A154B?logo=slack&logoColor=white&style=for-the-badge"/></a>
  <a href="https://matrix.org"><img src="https://img.shields.io/badge/Matrix-000000?logo=matrix&logoColor=white&style=for-the-badge"/></a>
  <a href="https://developers.line.biz/console/"><img src="https://img.shields.io/badge/LINE-00C300?logo=line&logoColor=white&style=for-the-badge"/></a>
  <a href="https://developers.facebook.com/docs/whatsapp/cloud-api"><img src="https://img.shields.io/badge/WhatsApp-25D366?logo=whatsapp&logoColor=white&style=for-the-badge"/></a>
  <img src="https://img.shields.io/badge/Webhook-6E7681?style=for-the-badge"/>
</div>

所有适配器在同一 `garudust-server` 进程中运行。在 `~/.garudust/.env` 中设置对应 token 即可自动激活。

---

## LLM 提供商

| 提供商 | `config.yaml` | `.env` |
|--------|--------------|--------|
| Anthropic | `provider: anthropic` | `ANTHROPIC_API_KEY` |
| OpenAI | `provider: openai` | `OPENAI_API_KEY` |
| Google Gemini | `provider: gemini` | `GEMINI_API_KEY` |
| Groq | `provider: groq` | `GROQ_API_KEY` |
| Mistral | `provider: mistral` | `MISTRAL_API_KEY` |
| DeepSeek | `provider: deepseek` | `DEEPSEEK_API_KEY` |
| xAI (Grok) | `provider: xai` | `XAI_API_KEY` |
| OpenRouter | `provider: openrouter` *（默认）* | `OPENROUTER_API_KEY` |
| AWS Bedrock | `provider: bedrock` | `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` |
| Ollama | `provider: ollama` + `base_url` | *（无需）* |
| vLLM | `provider: vllm` + `base_url` | `VLLM_API_KEY` |
| ThaiLLM | `provider: thaillm` | `THAILLM_API_KEY` |
| 任意 OpenAI 兼容 | `provider: custom` + `base_url` | 对应 API 密钥 |

备用密钥：`LLM_FALLBACK_API_KEYS=key2,key3` — 鉴权失败时自动轮换

---

## 工具

内置工具开箱即用，无需任何配置。

| 工具 | 说明 |
|------|------|
| `web_fetch` | 抓取 URL 内容 |
| `web_search` | 网页搜索（Brave / Serper / DuckDuckGo） |
| `http_request` | 自定义 HTTP 请求，支持自定义 headers 和 body |
| `browser` | 通过 CDP 控制 Chrome/Chromium — 点击、输入、截图、执行 JS |
| `read_file` / `write_file` | 文件读写 |
| `list_directory` | 支持 glob 模式和深度限制的目录列表 |
| `terminal` | 执行 shell 命令（可选 Docker 沙箱隔离） |
| `memory` | 跨会话持久化键值存储 |
| `session_search` | 全文搜索历史对话（FTS5 trigram） |
| `delegate_task` | 并行派生子智能体处理分解任务 |
| `skill_view` / `write_skill` | 加载和编写可复用技能 |

**自定义脚本工具** — 在 `~/.garudust/tools/<name>/` 中放置 `tool.yaml` 和脚本：

```yaml
# tool.yaml
name: get_weather
description: 获取城市当前天气
schema:
  type: object
  properties:
    city: { type: string }
  required: [city]
command: "curl -s wttr.in/{city}?format=3"
# env_required: [MY_API_KEY]   # 仅将指定密钥从 ~/.garudust/.env 转发给脚本
```

通过 `config.yaml` 为每个工具指定模型 — 以 `GARUDUST_MODEL` / `GARUDUST_FALLBACK_MODEL` 环境变量形式传递给脚本：

```yaml
tools:
  get_weather:
    model: groq/llama-3.1-8b-instant
    fallback_model: openrouter/meta-llama/llama-3.1-8b-instruct
```

**MCP** — 在 `config.yaml` 中接入任意 [Model Context Protocol](https://modelcontextprotocol.io) 服务器：

```yaml
mcp_servers:
  - name: filesystem
    command: npx
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
```

---

## Hub

一条命令，即可用社区工具和技能扩展智能体能力。

**Tool Hub**（[garudust-hub](https://github.com/garudust-org/garudust-hub)）

```bash
garudust tool list                        # 浏览可用工具
garudust tool install weather             # 下载到 ~/.garudust/tools/weather/
garudust tool install hash_text
garudust tool uninstall weather
garudust tool update                      # 更新所有 hub 工具
```

**Skills Hub**（[agentskills.io](https://agentskills.io)）

```bash
garudust skill list
garudust skill install agentskills-org/hub/git-workflow
garudust skill install https://example.com/skills/my-skill/SKILL.md
garudust skill uninstall git-workflow
```

---

## 记忆

智能体将所学内容保存至 `~/.garudust/memory/`，并在每次会话开始时自动加载 — 无需重复说明。可复用的工作流会被自动写入 `~/.garudust/skills/`，无需任何手动操作。

```
你：JSON 始终使用 2 个空格缩进
Agent：已记录。
# 下次会话：直接应用，无需再次提醒
```

---

## 参与贡献

Welcome, garudian！Garudust 由一群相信 AI 智能体应该快速、私密、受用户掌控的人共同构建。每一份贡献——无论是修复拼写错误、添加新工具还是实现完整特性——都让它变得更好。

### 贡献方式

| 方向 | 具体内容 | 难度 |
|------|---------|------|
| Bug 报告 | 提交 issue 并附上复现步骤 | 极低 |
| 文档 | 修复错别字、改进示例、翻译 | 低 |
| Hub 工具 | 向 [garudust-hub](https://github.com/garudust-org/garudust-hub) 添加脚本工具 | 低 |
| 技能 | 编写可复用技能并分享到 [agentskills.io](https://agentskills.io) | 低 |
| 平台适配器 | 在 `garudust-platforms` 中添加新聊天平台支持 | 中等 |
| 传输提供商 | 在 `garudust-transport` 中添加新 LLM 提供商 | 中等 |
| 核心功能 | 智能体循环、记忆、压缩、工具 | 较高 |

### 快速开始

```bash
git clone https://github.com/garudust-org/garudust-agent
cd garudust-agent
cargo build                   # 构建所有 crate
cargo test --workspace        # 运行完整测试套件
cargo clippy --workspace      # lint 检查
```

**添加内置工具** — 在 `crates/garudust-tools/src/toolsets/` 中实现 `Tool` trait，并在 `ToolRegistry::new()` 中注册。通常只需一个文件，不超过 100 行。

**添加 hub 工具** — 在 [garudust-hub](https://github.com/garudust-org/garudust-hub) 的 `tools/<name>/` 下放置 `tool.yaml` 和脚本，无需 Rust。

**添加 LLM 提供商** — 在 `crates/garudust-transport/src/` 中实现 `ProviderTransport`（`chat` + `chat_stream`），并在 `registry.rs` 中接入。

**添加平台适配器** — 在 `crates/garudust-platforms/src/` 中实现 `PlatformAdapter`（`send_message` + `start_listening`）。

详细分步指南请参见 [CONTRIBUTING.md](../../../CONTRIBUTING.md)。

### 社区

- [Discord](https://discord.com/channels/1501414298449088745/1501414298893942877) — 交流、提问与分享想法
- [Issues](https://github.com/garudust-org/garudust-agent/issues) — Bug 报告与功能请求
- [Discussions](https://github.com/garudust-org/garudust-agent/discussions) — 长篇提案与深度讨论

---

## 许可证

MIT — 详见 [LICENSE](../../../LICENSE)

---

## 贡献者

[![](https://contrib.rocks/image?repo=garudust-org/garudust-agent)](https://github.com/garudust-org/garudust-agent/graphs/contributors)

---

<div align="center">
  <img src="https://visitor-badge.laobi.icu/badge?page_id=garudust-org.garudust-agent&style=flat" alt="visitors"/>
</div>
