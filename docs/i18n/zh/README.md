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

**用 Rust 编写的可自托管、可自我进化的 AI 智能体运行时**

从终端聊天，连接 Telegram / Discord / Slack / Matrix / LINE，或通过 HTTP 调用 — 一个二进制文件搞定一切。它记住你教给它的东西，说你的语言，每次使用都变得更聪明。

---

## 为什么选择 Garudust？

- **二进制文件 ~10 MB，冷启动 < 20 ms** — 单一静态链接二进制文件，本地使用无需任何运行时依赖
- **自我进化** — 学习你的偏好，将可复用的工作流保存为技能，无需提醒两次便能自我修正
- **说你的语言** — 自动检测中文、泰语、日语、阿拉伯语、韩语等，无需任何配置
- **一个环境变量切换 LLM 提供商** — 支持 Anthropic、OpenRouter、AWS Bedrock、Ollama、vLLM 或任何 OpenAI 兼容端点
- **安全优先设计** — Docker 沙箱、无条件命令拦截、内存投毒防护，以及工具输出的自动密钥脱敏
- **随处运行** — 笔记本 TUI、无头服务器、Docker、Telegram、Discord、Slack、Matrix、LINE、HTTP
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
cd garudust-agent
cargo build --release
export PATH="$PATH:$(pwd)/target/release"
```

---

## 快速开始

```bash
garudust setup   # 首次配置向导 — 选择提供商并保存 API key
```

| | **1 — TUI** | **2 — 单次执行** | **3 — 服务器 / Docker** |
|---|---|---|---|
| **命令** | `garudust` | `garudust "任务"` | `garudust-server` |
| **适用场景** | 个人交互式聊天 | 脚本 / 管道 / CI | 机器人、API、定时任务 |
| **交互方式** | 终端 UI | stdout + 退出码 | HTTP + 聊天平台 |

### 1 — 交互式 TUI

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

### 2 — 单次执行

```bash
garudust "将过去 7 天的 git log 整理成 changelog"
```

输出到 stdout，成功时退出码为 0，可直接与管道配合使用。

### 3 — 服务器 / Docker

```bash
# 最简启动
garudust-server --port 3000

# 使用 Docker
echo "OPENROUTER_API_KEY=sk-or-..." > .env
docker compose up

# 生产环境：沙箱 + Telegram 机器人 + 每日定时任务
GARUDUST_TERMINAL_SANDBOX=docker \
GARUDUST_API_KEY=my-secret-token \
TELEGRAM_TOKEN=123:ABC \
GARUDUST_CRON_JOBS="0 9 * * *=向 Telegram 发送晨报" \
GARUDUST_MEMORY_CRON="0 3 * * *" \
garudust-server --port 3000 --approval-mode smart
```

---

## CLI 参考

```bash
garudust setup                              # 首次配置向导
garudust doctor                             # 检查 API key、连通性、数据库
garudust config show                        # 显示当前配置
garudust model                              # 显示当前模型，提示输入新模型
garudust model anthropic/claude-opus-4-7   # 直接切换模型
garudust config set ANTHROPIC_API_KEY sk-ant-...
garudust config set VLLM_BASE_URL http://localhost:8000/v1
```

---

## 配置

所有持久化设置保存在 `~/.garudust/config.yaml`。密钥和令牌保存在 `~/.garudust/.env` — 运行 `garudust setup` 进行交互式配置。两个文件均在启动时安全加载，不会转发给子进程。

### `~/.garudust/config.yaml`

```yaml
model: anthropic/claude-sonnet-4-6   # 模型标识符
provider: anthropic                  # 若省略则从 API key 自动检测

security:
  terminal_sandbox: docker           # none（默认）| docker
  terminal_sandbox_image: ubuntu:24.04
  terminal_sandbox_opts:
    - "--network=none"               # 切断容器内的出站网络访问
    - "--memory=512m"                # 限制内存用量

nudge_interval: 5                    # 每 N 次迭代提醒保存记忆（0 = 关闭）

mcp_servers:
  - name: filesystem
    command: npx
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
  - name: postgres
    command: npx
    args: ["-y", "@modelcontextprotocol/server-postgres", "postgresql://localhost/mydb"]
```

## 安全性

Garudust 在智能体拥有真实工具访问权限时仍能保持安全运行。

### 终端沙箱

设置 `terminal_sandbox: docker` 后，所有 shell 命令都在隔离容器内执行，默认使用严格参数：`--cap-drop ALL`、`--security-opt no-new-privileges:true`、`--pids-limit 256` 和临时 `/tmp`。当前工作目录挂载至 `/workspace`，文件操作仍可正常使用。

> **注意：** 必须安装并运行 Docker。若 Docker 未安装，启动时和工具调用时均会显示明确的错误提示。

### 命令硬性拦截

以下模式无条件被拦截，与审批模式和沙箱配置无关：

| 模式 | 示例 |
|------|------|
| 递归删除根文件系统 | `rm -rf /`、`rm -rf /*` |
| 格式化文件系统 | `mkfs`、`mkfs.ext4 /dev/sda1` |
| Fork 炸弹 | `:(){ :|:& };:` |
| 写入原始块设备 | `dd of=/dev/sda`、`cat > /dev/nvme0n1` |
| 系统关机 / 重启 | `shutdown`、`reboot`、`halt`、`systemctl poweroff` |
| 写入凭证路径 | `~/.ssh/authorized_keys`、`~/.aws/credentials`、`~/.bashrc` |

### 审批模式

| 模式 | 行为 |
|------|------|
| `smart` *（默认）* | 批准所有工具；系统提示中的宪法约束是主要防线；所有破坏性调用均记录审计日志 |
| `auto` | 与 `smart` 相同 — 用于可信的自动化流水线 |
| `deny` | 无条件拦截所有破坏性工具调用 — 适合只读智能体 |

通过 `GARUDUST_APPROVAL_MODE` 或 `--approval-mode` 设置。

### 内存保护

从历史会话中检索的内存条目被包裹在 `<untrusted_memory>` 标签中，使模型将其视为用户控制的数据而非可信指令，防止内存投毒攻击。写入时的验证也会拒绝包含 XML 控制标签的条目。

### 输出脱敏

启动时加载的 API key 和密钥会在终端命令输出到达模型之前自动被清除。输出还会被截断至 50 KB（40% 头部 + 60% 尾部），以防止上下文泛滥。

---

## 记忆与自我进化

Garudust 跨会话记住信息，使用得越多越聪明。

### 记忆机制

智能体自动将持久知识保存到 `~/.garudust/memory/`：

```
你：JSON 始终使用 2 空格缩进
智能体：[保存记忆] 明白了，从现在起 JSON 将使用 2 空格缩进。
```

下次会话时，该偏好已经加载好了。内置提示每隔几次迭代触发一次，提醒智能体在会话结束前保存新发现的事实。通过 `config.yaml` 中的 `nudge_interval` 配置间隔（0 = 关闭）。

### 保存内容

| 类别 | 示例 |
|------|------|
| 偏好设置 | 输出格式、语言、语气、工具选择 |
| 项目详情 | 路径、配置、规范、已知的特殊行为 |
| 纠正内容 | 你告诉智能体停止做的事 — 立即保存 |

智能体**不会**保存会话日志、任务进度或一次性细节 — 只保存未来会话中有价值的事实。

---

## 技能（Skills）

技能是智能体在行动前加载的可复用指令集，存储在 `~/.garudust/skills/`，每次调用时热重载。

```
~/.garudust/skills/
  git-workflow/SKILL.md
  daily-standup/SKILL.md
  rust-code-review/SKILL.md
```

### 主动技能加载

处理每条消息前，智能体扫描所有可用技能。若某技能相关，它会调用 `skill_view` 加载完整指令再继续。无论你用何种语言写指令，既定工作流都会被遵循。

### 创建技能

智能体在发现多步骤工作流时会自动创建技能：

```
你：为 Rust PR 审查创建一个技能
智能体：[调用 write_skill] 已将技能保存到 ~/.garudust/skills/rust-code-review/SKILL.md
```

最小化 `SKILL.md`：

```markdown
---
name: git-workflow
description: 规范化的 Git 提交和 PR 工作流
version: 1.0.0
---

始终编写 conventional commits。推送前始终运行测试。
先开 draft PR，CI 通过后再标记为 ready。
```

### 更新技能

如果智能体发现技能步骤已过时或有误，它会立即修补文件，无需等待提醒。

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

在 `~/.garudust/.env` 中设置相关令牌并启动 `garudust-server`，所有适配器可在同一进程中同时运行。

| 平台 | 所需令牌 |
|------|---------|
| Telegram | `TELEGRAM_TOKEN` |
| Discord | `DISCORD_TOKEN` |
| Slack | `SLACK_BOT_TOKEN`、`SLACK_APP_TOKEN` |
| Matrix | `MATRIX_HOMESERVER`、`MATRIX_USER`、`MATRIX_PASSWORD` |
| LINE | `LINE_CHANNEL_TOKEN`、`LINE_CHANNEL_SECRET` |
| Webhook | 始终开启，监听 `POST /webhook` — 无需令牌 |

**Telegram** — 通过 [@BotFather](https://t.me/botfather) 创建机器人，复制 token。

**Discord** — 在 [discord.com/developers](https://discord.com/developers/applications) 创建应用，在 Bot 设置中启用 **Message Content Intent**，复制 token。

**Slack** — 在 [api.slack.com/apps](https://api.slack.com/apps) 创建应用，启用 **Socket Mode**，添加权限范围 `chat:write channels:history im:history`，安装到工作区。

**Matrix** — 支持任意 homeserver（matrix.org、Synapse、Dendrite 等）。

**LINE** — 在 [developers.line.biz](https://developers.line.biz/console/) 创建 Messaging API channel，复制 **Channel access token** 和 **Channel secret**，设置 `GARUDUST_LINE_PORT`（默认 `3002`），并在 LINE 控制台将 Webhook URL 设为 `https://your-host:3002/line`。

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

在 `~/.garudust/.env` 中设置对应的 key，然后通过 `garudust model` 或设置 `GARUDUST_MODEL` 切换模型。

---

## 内置工具

| 工具 | 描述 |
|------|------|
| `web_fetch` | 获取 URL 内容（静态页面） |
| `web_search` | 通过 Brave Search API 搜索（需 `BRAVE_SEARCH_API_KEY`） |
| `browser` | 通过 CDP 控制 Chrome/Chromium — 导航、点击、输入、截图、运行 JS |
| `read_file` | 从文件系统读取文件 |
| `write_file` | 向文件系统写入文件；敏感凭证路径始终被拦截 |
| `terminal` | 运行 shell 命令；设置 `terminal_sandbox: docker` 后在 Docker 沙箱中执行 |
| `memory` | 持久化键值记忆（add / read / replace / remove） |
| `user_profile` | 读取和更新持久化用户档案 |
| `session_search` | 跨历史对话全文搜索（SQLite FTS5） |
| `delegate_task` | 为分解的任务生成并行子智能体 |
| `skills_list` | 列出可用技能 |
| `skill_view` | 按名称加载技能完整指令 |
| `write_skill` | 在 `~/.garudust/skills/` 中创建或更新技能 |

**MCP 工具** — 通过在 `config.yaml` 的 `mcp_servers` 列表中添加条目，连接任意 [MCP](https://modelcontextprotocol.io) 服务器（见配置章节）。

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
  garudust-platforms   Telegram、Discord、Slack、Matrix、LINE、Webhook
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
- **新平台** — 实现 `PlatformAdapter`（如 Signal、WhatsApp）
- **改进 TUI** — 多行输入、语法高亮、鼠标支持
- **测试** — 集成测试、属性测试、快照测试

```bash
git clone https://github.com/garudust-org/garudust-agent
cd garudust-agent
cargo build
cargo test --workspace
cargo clippy --workspace --all-targets -- -W clippy::all -W clippy::pedantic
```

请阅读 [CONTRIBUTING.md](../../../CONTRIBUTING.md) 了解代码规范、提交约定和完整 CI 检查清单。

---

## 许可证

MIT — 详见 [LICENSE](../../../LICENSE)
