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

基于 Rust 构建的自进化 AI 智能体运行时 — 单一 ~10 MB binary，零运行时依赖。终端聊天、跨 7 大平台回复、开放 REST + WebSocket API。一个环境变量即可切换 LLM 提供商。无遥测，无供应商锁定。

<div align="center">
  <img src="../../../assets/demo.svg" alt="Garudust demo"/>
</div>

---

## 支持的操作系统

| 操作系统 | 架构 | 文件 |
|---------|------|------|
| macOS | Apple Silicon（M1/M2/M3/M4） | `garudust-*-aarch64-apple-darwin.tar.gz` |
| macOS | Intel | `garudust-*-x86_64-apple-darwin.tar.gz` |
| Linux | x86_64 | `garudust-*-x86_64-unknown-linux-musl.tar.gz` |
| Linux | ARM64（Raspberry Pi 4/5、Jetson） | `garudust-*-aarch64-unknown-linux-musl.tar.gz` |
| Windows | x86_64 | `garudust-*-x86_64-pc-windows-msvc.zip` |

从 [GitHub Releases](https://github.com/garudust-org/garudust-agent/releases/latest) 下载 — 无需安装 Rust。

---

## 快速开始

**01 — 安装**

从 [GitHub Releases](https://github.com/garudust-org/garudust-agent/releases/latest) 下载预构建 binary（macOS、Linux、Windows、ARM64）：

```bash
ARCH=$(uname -m)
[ "$ARCH" = "aarch64" ] && TARGET="aarch64-unknown-linux-musl" || TARGET="x86_64-unknown-linux-musl"
VER=$(curl -s https://api.github.com/repos/garudust-org/garudust-agent/releases/latest | grep tag_name | cut -d'"' -f4)
curl -LO "https://github.com/garudust-org/garudust-agent/releases/latest/download/garudust-${VER}-${TARGET}.tar.gz"
tar -xzf garudust-*.tar.gz && sudo mv garudust garudust-server /usr/local/bin/
```

或从源码构建（需 Rust 1.87+）：`git clone https://github.com/garudust-org/garudust-agent && cargo build --release`

---

**02 — 配置**

```bash
garudust setup    # 交互式向导 — 选择提供商，生成 config.yaml + .env
```

或直接将密钥写入 `~/.garudust/.env`（如 `ANTHROPIC_API_KEY=sk-ant-...`）。支持的提供商列表见 [LLM 提供商](#llm-提供商)。

---

**03 — 运行**

```bash
garudust                             # 交互式 TUI
garudust "整理 git log 为 changelog" # 单次任务
garudust --hint fast "这段代码对吗"  # 使用更廉价的模型
garudust-server --port 3000          # 无头 REST + WebSocket 服务器
docker compose up -d
```

<div align="center">
  <img src="../../../assets/demo-tui.png" alt="Garudust TUI" width="700"/>
</div>

| 按键 | 操作 |
|------|------|
| `Enter` | 发送消息 |
| `↑ ↓` | 滚动历史 |
| `/new` | 新会话 |
| `/model <name>` | 切换模型 |
| `Ctrl+C` | 退出 |

---

## 为什么选择 Garudust？

- **~10 MB，冷启动 < 20 ms** — 单一静态链接文件，零运行时依赖
- **自我进化** — 记住偏好，将重复工作流自动保存为技能，无需重复提醒
- **并行工具执行** — 独立工具并发运行，仅在必要时串行化
- **24 个 LLM 提供商** — Anthropic、OpenAI、Gemini、Groq、Ollama、Bedrock 等 — 一行配置即可切换
- **7 大平台适配器** — Telegram、Discord、Slack、Matrix、LINE、WhatsApp、Webhook，同进程运行
- **安全优先设计** — Docker 沙箱、RBAC、per-user 速率限制、自动脱敏

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

在 `config.yaml` 中设置 `providers.default.name`，并在 `~/.garudust/.env` 中填写对应密钥：

| 提供商 | `name` | `.env` key |
|--------|--------|------------|
| Anthropic | `anthropic` | `ANTHROPIC_API_KEY` |
| OpenAI | `openai` | `OPENAI_API_KEY` |
| Google Gemini | `gemini` | `GEMINI_API_KEY` |
| Groq | `groq` | `GROQ_API_KEY` |
| Mistral | `mistral` | `MISTRAL_API_KEY` |
| DeepSeek | `deepseek` | `DEEPSEEK_API_KEY` |
| xAI (Grok) | `xai` | `XAI_API_KEY` |
| OpenRouter | `openrouter` | `OPENROUTER_API_KEY` |
| AWS Bedrock | `bedrock` | `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` |
| Ollama | `ollama` | *（无需 — 自定义端点请添加 `url:`）* |
| vLLM | `vllm` | `VLLM_API_KEY` |
| ThaiLLM | `thaillm` | `THAILLM_API_KEY` |
| Together AI | `together` | `TOGETHER_API_KEY` |
| Fireworks AI | `fireworks` | `FIREWORKS_API_KEY` |
| Cerebras | `cerebras` | `CEREBRAS_API_KEY` |
| Perplexity | `perplexity` | `PERPLEXITY_API_KEY` |
| Cohere | `cohere` | `COHERE_API_KEY` |
| NVIDIA NIM | `nvidia` | `NVIDIA_API_KEY` |
| 阿里云百炼（DashScope） | `alibaba` | `DASHSCOPE_API_KEY` |
| 字节豆包 | `doubao` | `ARK_API_KEY` |
| 智谱 AI（GLM） | `zhipu` | `ZHIPU_API_KEY` |
| Moonshot（Kimi） | `moonshot` | `MOONSHOT_API_KEY` |
| 百度文心 | `baidu` | `QIANFAN_API_KEY` |
| 任意 OpenAI 兼容 | *（省略 `name:`，在 profile 中设置 `url:`）* | 对应 API 密钥 |

备用密钥：在 `.env` 中设置 `LLM_FALLBACK_API_KEYS=key2,key3` — 鉴权失败时自动轮换。

---

## 架构

```
┌──────────────────────────────────────────────────────────────────────┐
│  bin/garudust (CLI)              bin/garudust-server (守护进程)      │
└────────────────────┬─────────────────────────┬───────────────────────┘
                     │                         │
                     │          ┌──────────────┴───────────────────────┐
                     │          │  garudust-gateway  (仅服务端)        │
                     │          │  POST /chat · POST /stream · GET /ws │
                     │          │  RBAC · /join · /invite · Metrics    │
                     │          ├──────────────────────────────────────┤
                     │          │  garudust-platforms  (仅服务端)      │
                     │          │  Telegram · Discord · Slack          │
                     │          │  LINE · Matrix · WhatsApp · Webhook  │
                     │          ├──────────────────────────────────────┤
                     │          │  garudust-cron  (仅服务端)           │
                     │          │  cron 定时自主任务                   │
                     │          └──────────────┬───────────────────────┘
                     │                         │
                     ▼                         ▼
┌──────────────────────────────────────────────────────────────────────┐
│                    garudust-agent  (运行循环)                        │
│  加载记忆 → 构建 prompt → 调用 LLM → 执行工具 → 循环                │
└──────┬──────────────┬─────────────────┬─────────────────────────────┘
       ▼              ▼                 ▼
  garudust-      garudust-        garudust-
  transport      tools            memory
  (24 LLMs +    (内置 +          (memory.md +
  密钥轮换)      hub + MCP)       SQLite + RAG)

garudust-core — 共享类型 · 配置 · 特征（被以上所有 crate 使用）
```

---

## 配置

密钥 → `~/.garudust/.env`。其余配置 → `~/.garudust/config.yaml`。

### `~/.garudust/.env`

```bash
# LLM 提供商 — 设置一个（无 config.yaml 时自动从环境变量检测）
ANTHROPIC_API_KEY=sk-ant-...
# OPENAI_API_KEY=sk-...
# GEMINI_API_KEY=AIza...
# GROQ_API_KEY=gsk_...

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

# 搜索（可选 — 未设置时回退到 DuckDuckGo）
BRAVE_SEARCH_API_KEY=BSA...
SERPER_API_KEY=...

# 网关鉴权
GARUDUST_API_KEY=my-gateway-secret
```

### `~/.garudust/config.yaml`

```yaml
providers:
  default:
    name: anthropic          # 完整列表见上方 LLM 提供商表格
    key: ${ANTHROPIC_API_KEY}
    model: claude-sonnet-4-6

security:
  approval_mode: smart       # auto | smart | deny
  terminal_sandbox: none     # none | docker  ← 生产环境请使用 docker
  rate_limit_rpm: ~          # 每 IP 限制（~ = 不限）
  rate_limit_rpm_per_user: ~ # 每（平台, 用户 ID）限制

# 仅针对单次任务切换模型，不影响默认配置：
routing:
  fast: groq-fast/llama-3.1-8b-instant
  # 使用: garudust --hint fast "快速提问"
```

完整配置参考见 [CONTRIBUTING.md](../../../CONTRIBUTING.md)。

---

## 工具

内置工具，开箱即用：

`web_fetch` · `web_search` · `http_request` · `browser`（CDP）· `read_file` · `write_file` · `list_directory` · `terminal` · `memory` · `session_search` · `delegate_task` · `skill_view` · `write_skill` · `doc_ingest` · `doc_search`

**Hub** — 来自 [garudust-hub](https://github.com/garudust-org/garudust-hub) 的社区工具和技能：

```bash
garudust tool install hash_text    # 脚本工具 → ~/.garudust/tools/hash_text/
garudust tool install read_qr
garudust skill install weather     # Markdown 指令，无需子进程
garudust skill install fetch-title
```

**MCP** — 接入任意 [Model Context Protocol](https://modelcontextprotocol.io) 服务器：

```yaml
mcp_servers:
  - name: filesystem
    command: npx
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
```

**自定义工具** — 在 `~/.garudust/tools/<name>/` 中放置 `tool.yaml` 和脚本，支持任意语言。参见 [garudust-hub](https://github.com/garudust-org/garudust-hub) 中的示例。

---

## 访问控制

通过 `config.yaml` 中的 `roles:` 实现基于角色的访问控制。若尚未分配任何用户，第一个发送私信的人将自动获得 `admin` 角色。

```yaml
roles:
  default_role: member
  definitions:
    admin:    { approval_mode: auto }
    member:   { approval_mode: smart, allowed_toolsets: [web, files, memory], denied_tools: [bash] }
    readonly: { approval_mode: deny }
  users:
    telegram:
      "123456789": admin
```

运行时命令：`/whoami` · `/join [code]` · `/invite <role> [max_uses]` · `/role list|add|approve|remove`

> **生产环境：** 设置 `terminal_sandbox: docker` 以沙箱化 shell 执行，设置 `max_delegation_depth: 0` 以防止子智能体链式委派。

---

## 记忆与技能

智能体将所学内容保存至 `~/.garudust/memory/`，并在每次会话开始时自动加载 — 无需重复说明。达到 `auto_skill_threshold` 次迭代后，可复用的工作流会被自动写入 `~/.garudust/skills/`。

---

## 参与贡献

Garudust 基于 Rust 构建，设计上易于扩展。选择你感兴趣的方向：

| 方向 | 位置 | 难度 |
|------|------|------|
| Hub 工具或技能 | [garudust-hub](https://github.com/garudust-org/garudust-hub) — `tool.yaml` + 脚本 | 低 — 无需 Rust |
| Bug 报告 / 文档 | [Issues](https://github.com/garudust-org/garudust-agent/issues) | 极低 |
| 新 LLM 提供商 | `crates/garudust-transport/src/` — impl `ProviderTransport`（2 个方法） | 中等 |
| 新平台适配器 | `crates/garudust-platforms/src/` — impl `PlatformAdapter`（2 个方法） | 中等 |
| 内置工具 | `crates/garudust-tools/src/toolsets/` — impl `Tool`，在 `ToolRegistry::new()` 中注册 | 中等（~100 行） |
| 核心功能 | Agent 循环、记忆、压缩、网关 | 较高 |

```bash
git clone https://github.com/garudust-org/garudust-agent
cd garudust-agent
cargo build && cargo test --workspace && cargo clippy --workspace
```

各方向详细指南：[CONTRIBUTING.md](../../../CONTRIBUTING.md)

**社区：** [Discord](https://discord.com/channels/1501414298449088745/1501414298893942877) · [Issues](https://github.com/garudust-org/garudust-agent/issues) · [Discussions](https://github.com/garudust-org/garudust-agent/discussions) · [dev.to/garudust](https://dev.to/garudust)

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
