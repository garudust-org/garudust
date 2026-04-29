<div align="center">
  <img src="../../../assets/logo.png" alt="Garudust" width="260"/>

  <a href="../../../README.md"><img src="https://img.shields.io/badge/🇺🇸-English-blue?style=flat-square" alt="English"/></a>
  <a href="../th/README.md"><img src="https://img.shields.io/badge/🇹🇭-ภาษาไทย-red?style=flat-square" alt="ภาษาไทย"/></a>
  <a href="../zh/README.md"><img src="https://img.shields.io/badge/🇨🇳-简体中文-yellow?style=flat-square" alt="简体中文"/></a>
</div>

# Garudust Agent

[![CI](https://github.com/garudust-org/garudust-agent/actions/workflows/ci.yml/badge.svg)](https://github.com/garudust-org/garudust-agent/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/garudust-org/garudust-agent)](https://github.com/garudust-org/garudust-agent/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](../../../LICENSE)
![Rust 1.75+](https://img.shields.io/badge/rust-1.75+-orange.svg)

**用 Rust 编写的可自托管 AI 智能体运行时**

从终端聊天，连接 Telegram / Discord / Slack / Matrix，或通过 HTTP 调用 — 一个二进制文件搞定一切。

---

## 为什么选择 Garudust？

大多数 AI 智能体框架基于 Python，体积庞大，启动缓慢。Garudust 不同：

- **二进制文件 ~10 MB，冷启动 < 20 ms** — 无需 Python 运行时，本地使用无需 Docker
- **一个环境变量切换 LLM 提供商** — 支持 Anthropic、OpenRouter、AWS Bedrock、Ollama、vLLM 或任何 OpenAI 兼容端点
- **随处运行** — 笔记本 TUI、无头服务器、Docker、Telegram、Discord、Slack、Matrix、HTTP
- **高度可组合** — 每个模块都是独立 crate，添加工具、平台或传输层无需改动其他代码

---

## 安装

### 预构建二进制文件（推荐）

从 [**GitHub Releases**](https://github.com/garudust-org/garudust-agent/releases/latest) 下载 — 无需安装 Rust：

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

### 从源码构建

需要 Rust 1.75+：

```bash
git clone https://github.com/garudust-org/garudust-agent
cd garudust
cargo build --release
export PATH="$PATH:$(pwd)/target/release"
```

---

## 快速开始

### 1. 配置并聊天

```bash
garudust setup   # 选择提供商（OpenRouter / Anthropic / vLLM / Ollama / 自定义）并保存 key
garudust         # 启动交互式 TUI
```

### 2. Docker（服务器模式）

```bash
# 云端提供商
echo "OPENROUTER_API_KEY=sk-or-..." > .env
docker compose up

# 通过 Ollama 使用本地模型
echo "OLLAMA_BASE_URL=http://host.docker.internal:11434" > .env
echo "GARUDUST_MODEL=llama3.2" >> .env
docker compose up
```

```bash
curl -X POST http://localhost:3000/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "2+2 等于多少？"}'
```

---

## CLI 用法

### 交互式 TUI

```bash
garudust
```

| 按键 | 操作 |
|------|------|
| `Enter` | 发送消息 |
| `↑ ↓` | 滚动历史记录 |
| `/new` | 清除历史，开始新会话 |
| `/model <名称>` | 运行时切换模型 |
| `/help` | 显示所有斜杠命令 |
| `Ctrl+C` | 退出 |

### 单次任务

```bash
garudust "将过去 7 天的 git log 整理成 changelog"
garudust --model anthropic/claude-opus-4-7 "对这个 PR 进行安全审查"
```

### 配置命令

```bash
garudust setup                              # 首次配置向导（Quick 或 Full 模式）
garudust doctor                             # 检查 API key、连通性、数据库
garudust config show                        # 显示当前配置
garudust model                              # 显示当前模型，提示输入新模型
garudust model anthropic/claude-opus-4-7   # 直接切换模型
garudust config set OPENROUTER_API_KEY sk-or-...
garudust config set ANTHROPIC_API_KEY sk-ant-...
garudust config set VLLM_BASE_URL http://localhost:8000/v1
```

---

## 无头服务器

`garudust-server` 在单个进程中运行 HTTP 网关、所有平台适配器和定时任务。

```bash
garudust-server --anthropic-key sk-ant-... --port 3000
```

### HTTP API

```bash
# 阻塞模式
curl -X POST http://localhost:3000/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "写一首关于 Rust 的俳句"}'

# 流式传输（Server-Sent Events）
curl -X POST http://localhost:3000/chat/stream \
  -H "Content-Type: application/json" \
  -d '{"message": "用 3 句话解释 async/await"}'

# WebSocket：ws://localhost:3000/chat/ws
# 发送：{"message": "你的任务"}  接收：文本片段… 然后 {"done":true}

# 健康检查与指标
curl http://localhost:3000/health
curl http://localhost:3000/metrics   # Prometheus 兼容
```

---

## 平台适配器

设置相关环境变量并启动 `garudust-server`，所有适配器可在同一进程中同时运行。

| 平台 | 所需环境变量 |
|------|------------|
| Telegram | `TELEGRAM_TOKEN` |
| Discord | `DISCORD_TOKEN` |
| Slack | `SLACK_BOT_TOKEN`、`SLACK_APP_TOKEN` |
| Matrix | `MATRIX_HOMESERVER`、`MATRIX_USER`、`MATRIX_PASSWORD` |
| Webhook | 始终开启，监听 `POST /webhook` — 无需 token |

**Telegram** — 通过 [@BotFather](https://t.me/botfather) 创建机器人，复制 token。

**Discord** — 在 [discord.com/developers](https://discord.com/developers/applications) 创建应用，在 Bot 设置中启用 **Message Content Intent**，复制 token。

**Slack** — 在 [api.slack.com/apps](https://api.slack.com/apps) 创建应用，启用 **Socket Mode**，添加权限范围 `chat:write channels:history im:history`，安装到工作区。

**Matrix** — 支持任意 homeserver（matrix.org、Synapse、Dendrite 等）。

```bash
TELEGRAM_TOKEN=123:ABC \
SLACK_BOT_TOKEN=xoxb-... \
SLACK_APP_TOKEN=xapp-... \
garudust-server --anthropic-key sk-ant-...
```

---

## LLM 提供商

| 提供商 | 选择方式 | 备注 |
|--------|---------|------|
| Anthropic | 设置 `ANTHROPIC_API_KEY` | 直接使用 Messages API |
| OpenRouter | 设置 `OPENROUTER_API_KEY` *（默认）* | 200+ 模型 |
| AWS Bedrock | 设置 `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` | Converse API，SigV4 |
| OpenAI Responses | `garudust config set provider codex` | `/v1/responses` 端点 |
| Ollama | 设置 `OLLAMA_BASE_URL` | 本地运行，无需 key |
| vLLM | 设置 `VLLM_BASE_URL` | 本地 OpenAI 兼容服务器 |
| 其他 OpenAI 兼容 | 设置 `GARUDUST_BASE_URL` | 通用传输层 |

```bash
# Ollama（本地，无需 key）
OLLAMA_BASE_URL=http://localhost:11434
GARUDUST_MODEL=llama3.2

# vLLM
VLLM_BASE_URL=http://localhost:8000/v1
VLLM_API_KEY=token-abc123          # 仅当服务器需要 --api-key 时填写
GARUDUST_MODEL=meta-llama/Llama-3.1-8B-Instruct

# AWS Bedrock
garudust config set provider bedrock
garudust config set model anthropic.claude-3-5-sonnet-20241022-v2:0
```

---

## 内置工具

| 工具 | 描述 |
|------|------|
| `web_fetch` | 获取 URL 内容（静态页面） |
| `web_search` | 通过 Brave Search API 搜索（需 `BRAVE_SEARCH_API_KEY`） |
| `browser` | 通过 CDP 控制 Chrome/Chromium — 导航、点击、输入、截图、运行 JS |
| `read_file` | 从文件系统读取文件 |
| `write_file` | 向文件系统写入文件 |
| `terminal` | 运行 shell 命令 |
| `memory` | 持久化键值内存（add / read / replace / remove） |
| `session_search` | 跨历史对话全文搜索（SQLite FTS5） |
| `delegate_task` | 为分解的任务生成并行子智能体 |
| `skills_list` | 列出可用技能 |
| `skill_view` | 按名称加载技能指令 |

### MCP 工具

在 `~/.garudust/config.yaml` 中连接任意 [MCP](https://modelcontextprotocol.io) 服务器：

```yaml
mcp_servers:
  - name: filesystem
    command: npx
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
  - name: postgres
    command: npx
    args: ["-y", "@modelcontextprotocol/server-postgres", "postgresql://localhost/mydb"]
```

---

## 技能（Skills）

技能是存储在 `~/.garudust/skills/` 中的可复用指令集，每次调用时从磁盘读取 — 修改文件后，下次调用立即生效。

```
~/.garudust/skills/
  git-workflow/SKILL.md
  daily-standup/SKILL.md
```

最小化 `SKILL.md` 示例：

```markdown
---
name: git-workflow
description: 规范化的 Git 提交和 PR 工作流
version: 1.0.0
---

始终编写 conventional commits。推送前始终运行测试...
```

---

## 全部环境变量

| 变量 | 默认值 | 描述 |
|------|--------|------|
| `ANTHROPIC_API_KEY` | — | Anthropic key（自动选择 Anthropic 传输层） |
| `OPENROUTER_API_KEY` | — | OpenRouter key（默认提供商） |
| `OLLAMA_BASE_URL` | — | Ollama base URL — 自动选择 Ollama，无需 key |
| `VLLM_BASE_URL` | — | vLLM base URL — 自动选择 vLLM 传输层 |
| `VLLM_API_KEY` | — | vLLM API key（可选） |
| `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` | — | Bedrock 凭证 |
| `BRAVE_SEARCH_API_KEY` | — | 启用 `web_search` 工具 |
| `GARUDUST_MODEL` | `anthropic/claude-sonnet-4-6` | 模型标识符 |
| `GARUDUST_PORT` | `3000` | HTTP 网关端口 |
| `GARUDUST_WEBHOOK_PORT` | `3001` | Webhook 适配器端口（`0` = 禁用） |
| `GARUDUST_BASE_URL` | — | 覆盖 LLM base URL（任何 OpenAI 兼容端点） |
| `GARUDUST_API_KEY` | — | `/chat*` 端点的 Bearer token（生产环境推荐） |
| `GARUDUST_APPROVAL_MODE` | `smart` | 命令审批：`auto` \| `smart` \| `deny` |
| `GARUDUST_RATE_LIMIT` | — | 每 IP 速率限制（请求数/分钟） |
| `TELEGRAM_TOKEN` | — | Telegram 机器人 token |
| `DISCORD_TOKEN` | — | Discord 机器人 token |
| `SLACK_BOT_TOKEN` | — | Slack 机器人 token（`xoxb-…`） |
| `SLACK_APP_TOKEN` | — | Slack Socket Mode app token（`xapp-…`） |
| `MATRIX_HOMESERVER` | — | Matrix homeserver URL |
| `MATRIX_USER` | — | Matrix 用户名（`@bot:matrix.org`） |
| `MATRIX_PASSWORD` | — | Matrix 密码 |
| `GARUDUST_CRON_JOBS` | — | 逗号分隔的 `"cron_expr=task"` 对 |
| `RUST_LOG` | `info` | 日志级别（`debug` 获取详细输出） |

### 定时任务

```bash
GARUDUST_CRON_JOBS="0 9 * * *=撰写晨报并保存到 ~/briefing.md" \
garudust-server --anthropic-key sk-ant-...
```

---

## 架构

```
┌──────────────────────────────────────────────────────────────────┐
│                        garudust-server                           │
│                                                                  │
│  HTTP /chat ────┐                                                │
│  HTTP /stream   │                                                │
│  WebSocket ─────┼──► GatewayHandler ──► ArcSwap<Agent>          │
│  Telegram       │                            │                   │
│  Discord        │                            ▼                   │
│  Slack ─────────┘                       run_loop()               │
│  Matrix                                  │         │             │
│  Cron ──────────────────────────►   Transport   ToolRegistry     │
│                                    (Anthropic    (web, browser,  │
│                                     OpenRouter   file, terminal, │
│                                     Bedrock      memory, MCP,    │
│                                     Codex        delegate, ...)  │
│                                     Ollama                       │
│                                     vLLM)                        │
└──────────────────────────────────────────────────────────────────┘
```

### Crate 布局

```
crates/
  garudust-core        共享 trait 和类型 — 零 I/O
  garudust-transport   LLM 适配器：Anthropic、OpenAI-compat、Codex、Bedrock、Ollama、vLLM
  garudust-tools       工具注册表 + 内置工具集（web、browser、file 等）
  garudust-memory      FileMemoryStore（markdown）+ SessionDb（SQLite + FTS5）
  garudust-agent       Agent 运行循环、上下文压缩器、提示构建器
  garudust-platforms   Telegram、Discord、Slack、Matrix、Webhook
  garudust-cron        定时调度器
  garudust-gateway     axum HTTP 网关 — /chat、/chat/stream、/chat/ws、/metrics

bin/
  garudust             CLI：交互式 TUI、单次任务、setup、doctor、config、model
  garudust-server      无头模式：所有平台 + HTTP + 定时任务，单进程运行
```

---

## 贡献

Garudust 设计为易于扩展 — 添加工具、传输层或平台适配器通常只需修改一个 crate，代码不超过 100 行。

### 新手入门议题

- **新工具** — 在 `garudust-tools` 中将任意 CLI 或 API 封装为 `Tool` 实现
- **新平台** — 实现 `PlatformAdapter`（如 Signal、LINE、WhatsApp）
- **改进 TUI** — 多行输入、语法高亮、鼠标支持
- **测试** — 集成测试、属性测试、快照测试

```bash
git clone https://github.com/garudust-org/garudust-agent
cd garudust
cargo build
cargo test --workspace
cargo clippy --workspace --all-targets -- -W clippy::all -W clippy::pedantic
```

请阅读 [CONTRIBUTING.md](../../../CONTRIBUTING.md) 了解代码规范、提交约定和完整 CI 检查清单。

---

## 许可证

MIT — 详见 [LICENSE](../../../LICENSE)
