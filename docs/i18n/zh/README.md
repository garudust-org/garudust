<div align="center">
  <img src="../../../assets/logo.jpg" alt="Garudust" width="260"/>

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

基于 Rust 构建的自进化 AI 智能体运行时 — 以单一 ~10 MB binary 交付，无任何运行时依赖。一个 binary 处理一切：终端聊天、跨多平台回复（Telegram、Discord、Slack、LINE、WhatsApp），或开放 REST + WebSocket API。通过 Tool Hub 即时扩展能力，或直接放置 YAML 文件添加自定义工具。连接任意 MCP 服务器，或让智能体自行编写并优化可复用的技能。无遥测、无供应商锁定 — 数据仅发送至您所选择的 LLM 提供商。

### 演示

<div align="center">
  <img src="../../../assets/demo.svg" alt="Garudust demo"/>
</div>

---

## 为什么选择 Garudust？

- **二进制文件 ~10 MB，冷启动 < 20 ms** — 单一静态链接二进制文件，本地使用无需任何运行时依赖
- **自我进化** — 学习你的偏好，将可复用的工作流保存为技能，无需提醒两次便能自我修正
- **兼容 agentskills.io 标准** — 一条命令从 [agentskills.io](https://agentskills.io) hub 或任意 GitHub 仓库安装技能；`allowed-tools`、版本锁定与 `scripts/` 执行开箱即用
- **Tool Hub 一键安装** — 用 `garudust tool install <name>` 即可浏览并安装社区工具，无需手动管理文件夹
- **说你的语言** — 自动检测中文、泰语、日语、阿拉伯语、韩语等，无需任何配置
- **一个环境变量切换 LLM 提供商** — 支持 Anthropic、OpenRouter、AWS Bedrock、Ollama、vLLM 或任何 OpenAI 兼容端点
- **安全优先设计** — Docker 沙箱、无条件命令拦截、内存投毒防护，以及工具输出的自动密钥脱敏
- **随处运行** — 笔记本 TUI、无头服务器、Docker、Telegram、Discord、Slack、Matrix、LINE、WhatsApp、HTTP
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

需要 Rust 1.87+：

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
garudust         # 启动 TUI 对话界面
```

### 1 — 交互式 TUI

```bash
garudust
```

<div align="center">
  <img src="../../../assets/demo-tui.png" alt="Garudust TUI" width="800"/>
</div>

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

### 3 — 服务器 / Docker with Platforms

```bash
# 最简启动
garudust-server --port 3000

# 使用 Docker
echo "OPENROUTER_API_KEY=sk-or-..." > .env
docker compose up

# 生产环境：沙箱 + LINE 机器人 + 每日定时任务
GARUDUST_TERMINAL_SANDBOX=docker \
GARUDUST_API_KEY=my-secret-token \
LINE_CHANNEL_TOKEN=<channel-access-token> \
LINE_CHANNEL_SECRET=<32-char-hex-secret> \
GARUDUST_CRON_JOBS="0 9 * * *=向 LINE 发送晨报" \
GARUDUST_MEMORY_CRON="0 3 * * *" \
garudust-server --port 3000 --approval-mode smart

# 通过 ngrok 暴露 LINE webhook（开发环境）
ngrok http 3002
# Webhook URL: https://xxxx.ngrok-free.app/line  ← 填入 LINE Developers Console
```

<div align="center">
  <img src="../../../assets/demo-line.jpg" alt="LINE Demo" width="420"/>
</div>

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

### 平台配置

#### Telegram 机器人

```bash
# ~/.garudust/.env
ANTHROPIC_API_KEY=sk-ant-...
TELEGRAM_TOKEN=123456789:AAFxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx

# 启动
garudust-server --telegram-token $TELEGRAM_TOKEN --anthropic-key $ANTHROPIC_API_KEY
```

#### LINE Messaging API

```bash
# ~/.garudust/.env
OPENROUTER_API_KEY=sk-or-...
LINE_CHANNEL_TOKEN=<channel-access-token>
LINE_CHANNEL_SECRET=<32位十六进制密钥>

# 启动（Webhook 接收地址：https://your-host:3002/line）
garudust-server \
  --api-key $OPENROUTER_API_KEY \
  --line-channel-token $LINE_CHANNEL_TOKEN \
  --line-channel-secret $LINE_CHANNEL_SECRET \
  --line-port 3002
```

#### WhatsApp Business

```bash
# ~/.garudust/.env
ANTHROPIC_API_KEY=sk-ant-...
WHATSAPP_ACCESS_TOKEN=EAAxxxxxxx
WHATSAPP_PHONE_NUMBER_ID=123456789012345
WHATSAPP_VERIFY_TOKEN=my_verify_token
WHATSAPP_APP_SECRET=<32位十六进制密钥>   # 可选 — 留空则跳过 HMAC 验证

# 启动（Webhook 接收地址：https://your-host:3003/whatsapp）
garudust-server \
  --anthropic-key $ANTHROPIC_API_KEY \
  --whatsapp-access-token $WHATSAPP_ACCESS_TOKEN \
  --whatsapp-phone-number-id $WHATSAPP_PHONE_NUMBER_ID \
  --whatsapp-verify-token $WHATSAPP_VERIFY_TOKEN \
  --whatsapp-app-secret $WHATSAPP_APP_SECRET \
  --whatsapp-port 3003
```

#### 多平台同时运行（Telegram + LINE + WhatsApp + HTTP Webhook）

所有适配器运行在同一进程中 — 设置所需令牌，其余平台自动跳过。

```bash
# ~/.garudust/.env
ANTHROPIC_API_KEY=sk-ant-...
TELEGRAM_TOKEN=123456789:AAFxxx
LINE_CHANNEL_TOKEN=<token>
LINE_CHANNEL_SECRET=<secret>
WHATSAPP_ACCESS_TOKEN=EAAxxx
WHATSAPP_PHONE_NUMBER_ID=123456789012345
WHATSAPP_VERIFY_TOKEN=my_verify_token

garudust-server \
  --anthropic-key      $ANTHROPIC_API_KEY \
  --telegram-token     $TELEGRAM_TOKEN \
  --line-channel-token $LINE_CHANNEL_TOKEN \
  --line-channel-secret $LINE_CHANNEL_SECRET \
  --whatsapp-access-token    $WHATSAPP_ACCESS_TOKEN \
  --whatsapp-phone-number-id $WHATSAPP_PHONE_NUMBER_ID \
  --whatsapp-verify-token    $WHATSAPP_VERIFY_TOKEN \
  --webhook-port 3001 \
  --line-port    3002 \
  --whatsapp-port 3003
```

> **提示：** 使用 `garudust setup`（模式 2 — Full）通过交互向导自动写入 `~/.garudust/.env`。

## 安全性

### 终端沙箱

在 `config.yaml` 中设置 `terminal_sandbox: docker`，使每条 shell 命令在隔离容器内执行（`--cap-drop ALL`、`--pids-limit 256`，工作目录挂载至 `/workspace`）。需要安装 Docker。

### 命令硬性拦截

无条件拦截，与审批模式无关：

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
| `smart` *（默认）* | 允许所有工具；宪法约束是主要防线；破坏性调用均记录审计日志 |
| `auto` | 与 `smart` 相同 — 用于可信的自动化流水线 |
| `deny` | 拦截所有破坏性调用 — 适合只读智能体 |

通过 `GARUDUST_APPROVAL_MODE` 或 `--approval-mode` 设置。

历史会话的内存条目被包裹在 `<untrusted_memory>` 标签中，以防止内存投毒攻击。API key 会自动从工具输出中清除；输出截断至 50 KB 以防止上下文泛滥。

---

## 记忆与自我进化

智能体将持久知识保存到 `~/.garudust/memory/`，并在每次会话开始时加载 — 无需重复说明：

```
你：JSON 始终使用 2 空格缩进
智能体：[保存记忆] 明白了，从现在起 JSON 将使用 2 空格缩进。
```

| 类别 | 示例 |
|------|------|
| 偏好设置 | 输出格式、语言、语气、工具选择 |
| 项目详情 | 路径、配置、规范、已知的特殊行为 |
| 纠正内容 | 你告诉智能体停止做的事 — 立即保存 |

通过 `config.yaml` 中的 `nudge_interval` 配置记忆保存提醒间隔（0 = 关闭）。

---

## 技能（Skills）

存储在 `~/.garudust/skills/` 的可复用指令集，每次调用时热重载。

```
~/.garudust/skills/
  git-workflow/SKILL.md
  daily-standup/SKILL.md
  rust-code-review/SKILL.md
```

智能体在每条消息前扫描所有技能并加载相关技能，发现或纠正工作流时自动创建和修补技能文件。

Garudust 兼容 [agentskills.io](https://agentskills.io) 开放标准 — 技能无需修改即可直接使用，包括 `allowed-tools` 权限限制与 `scripts/` 脚本执行。

一条命令从 agentskills.io hub 或任意 GitHub 仓库安装技能：

```bash
# 从 GitHub（owner/repo/path）
garudust skill install agentskills-org/hub/git-workflow

# 从直接 URL
garudust skill install https://example.com/skills/my-skill/SKILL.md

# 从 well-known 端点
garudust skill install well-known:https://example.com --name my-skill

garudust skill list                      # 查看已安装的技能
garudust skill uninstall git-workflow    # 删除技能
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

<div align="center">
  <a href="https://core.telegram.org/bots"><img src="https://img.shields.io/badge/Telegram-2CA5E0?logo=telegram&logoColor=white&style=for-the-badge" alt="Telegram"/></a>
  <a href="https://discord.com/developers/applications"><img src="https://img.shields.io/badge/Discord-5865F2?logo=discord&logoColor=white&style=for-the-badge" alt="Discord"/></a>
  <a href="https://api.slack.com/apps"><img src="https://img.shields.io/badge/Slack-4A154B?logo=slack&logoColor=white&style=for-the-badge" alt="Slack"/></a>
  <a href="https://matrix.org"><img src="https://img.shields.io/badge/Matrix-000000?logo=matrix&logoColor=white&style=for-the-badge" alt="Matrix"/></a>
  <a href="https://developers.line.biz/console/"><img src="https://img.shields.io/badge/LINE-00C300?logo=line&logoColor=white&style=for-the-badge" alt="LINE"/></a>
  <a href="https://developers.facebook.com/docs/whatsapp/cloud-api"><img src="https://img.shields.io/badge/WhatsApp-25D366?logo=whatsapp&logoColor=white&style=for-the-badge" alt="WhatsApp"/></a>
  <img src="https://img.shields.io/badge/Webhook-6E7681?style=for-the-badge" alt="Webhook"/>
</div>

在 `~/.garudust/.env` 中设置相关令牌并启动 `garudust-server`，所有适配器可在同一进程中同时运行。

| 平台 | 所需令牌 |
|------|---------|
| Telegram | `TELEGRAM_TOKEN` |
| Discord | `DISCORD_TOKEN` |
| Slack | `SLACK_BOT_TOKEN`、`SLACK_APP_TOKEN` |
| Matrix | `MATRIX_HOMESERVER`、`MATRIX_USER`、`MATRIX_PASSWORD` |
| LINE | `LINE_CHANNEL_TOKEN`、`LINE_CHANNEL_SECRET` |
| WhatsApp | `WHATSAPP_ACCESS_TOKEN`、`WHATSAPP_PHONE_NUMBER_ID`、`WHATSAPP_VERIFY_TOKEN` |
| Webhook | 始终开启，监听 `POST /webhook` — 无需令牌 |

**Telegram** — 通过 [@BotFather](https://t.me/botfather) 创建机器人，复制 token。

**Discord** — 在 [discord.com/developers](https://discord.com/developers/applications) 创建应用，在 Bot 设置中启用 **Message Content Intent**，复制 token。

**Slack** — 在 [api.slack.com/apps](https://api.slack.com/apps) 创建应用，启用 **Socket Mode**，添加权限范围 `chat:write channels:history im:history`，安装到工作区。

**Matrix** — 支持任意 homeserver（matrix.org、Synapse、Dendrite 等）。

**LINE** — 在 [developers.line.biz](https://developers.line.biz/console/) 创建 Messaging API channel，复制 **Channel access token** 和 **Channel secret**，设置 `GARUDUST_LINE_PORT`（默认 `3002`），并在 LINE 控制台将 Webhook URL 设为 `https://your-host:3002/line`。

**WhatsApp** — 在 [developers.facebook.com](https://developers.facebook.com/) 创建 Meta 应用并添加 **WhatsApp** 产品，复制 **Access token** 和 **Phone number ID**。设置 `GARUDUST_WHATSAPP_PORT`（默认 `3003`），并在 Meta 控制台将 Webhook URL 设为 `https://your-host:3003/whatsapp`。如需启用 HMAC 签名验证，还需设置 `WHATSAPP_APP_SECRET`。

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
| `web_search` | 搜索网页 — 优先使用 Serper（Google，需 `SERPER_API_KEY`），其次 Brave Search（需 `BRAVE_SEARCH_API_KEY`），最后回退到 DuckDuckGo |
| `browser` | 通过 CDP 控制 Chrome/Chromium — 导航、点击、输入、截图、运行 JS |
| `read_file` | 从文件系统读取文件 |
| `write_file` | 向文件系统写入文件；敏感凭证路径始终被拦截 |
| `list_directory` | 列出文件和目录；支持 glob 模式（`**/*.rs`）和深度限制 |
| `terminal` | 运行 shell 命令；设置 `terminal_sandbox: docker` 后在 Docker 沙箱中执行 |
| `memory` | 持久化键值记忆（add / read / replace / remove） |
| `user_profile` | 读取和更新持久化用户档案 |
| `session_search` | 跨历史对话全文搜索（SQLite FTS5） |
| `delegate_task` | 为分解的任务生成并行子智能体 |
| `skills_list` | 列出可用技能 |
| `skill_view` | 按名称加载技能完整指令 |
| `write_skill` | 在 `~/.garudust/skills/` 中创建或更新技能 |

**MCP 工具** — 通过在 `config.yaml` 的 `mcp_servers` 列表中添加条目，连接任意 [MCP](https://modelcontextprotocol.io) 服务器（见配置章节）。

**脚本工具** — 无需编写 Rust 即可添加自定义工具。将包含 `tool.yaml` 和可选脚本的文件夹放入 `~/.garudust/tools/`，然后重启 agent：

```
~/.garudust/tools/
└── get_weather/
    ├── tool.yaml   ← 名称、描述、schema、命令
    └── run.py      ← 脚本，在 command 中以 ./run.py 引用（可选）
```

```yaml
# tool.yaml
name: get_weather
description: 获取某城市的当前天气
destructive: false
schema:
  type: object
  properties:
    city:
      type: string
  required: [city]
command: "curl -s wttr.in/{city}?format=3"
```

参数会自动进行 shell 引号转义。命令在 tool 文件夹内运行，并设置 `$TOOL_DIR` 环境变量，因此 `./run.py` 及同目录文件均可正确解析。

### Tool Hub

通过一条命令从 [garudust-hub](https://github.com/garudust-org/garudust-hub) 安装社区构建的脚本工具 — 无需手动创建文件夹：

```bash
garudust tool list                  # 浏览可用工具和已安装工具
garudust tool install weather       # 下载到 ~/.garudust/tools/weather/
garudust tool install hash_text
garudust tool uninstall weather     # 移除工具及其文件夹
garudust tool update                # 将所有 hub 工具更新至最新版本
```

`garudust tool list` 同时显示运行时依赖和工具描述：

```
NAME              INSTALLED  AVAILABLE  REQUIRES  DESCRIPTION
----------------------------------------------------------------------
csv_to_json       1.0.0      1.0.0      python3   Convert a CSV file to a JSON array…
fetch_title       —          1.0.0      python3   Fetch the HTML title of a webpage…
hash_text         —          1.0.0      —         Compute the SHA-256 hash of a string
markdown_to_html  —          1.0.0      rust      Convert a Markdown file to HTML…
read_qr           1.0.0      1.0.0      bash      Decode a QR code from an image file
weather           —          1.0.0      bash      Get current weather for a city…
yaml_to_json      —          1.0.0      node      Convert a YAML file to formatted JSON
```

已安装的工具记录在 `~/.garudust/tools/registry.json` 中，每次 agent 启动时与手动创建的工具一起自动加载。

| 命令 | 描述 |
|------|------|
| `tool list` | 并列显示已安装工具和 hub 可用工具 |
| `tool list --offline` | 仅显示本地已安装工具（不发起网络请求） |
| `tool install <名称>` | 从 hub 下载到 `~/.garudust/tools/<名称>/` |
| `tool install <名称> --hub <owner/repo>` | 从自定义 hub 仓库安装 |
| `tool uninstall <名称>` | 删除工具文件夹和注册表条目 |
| `tool update` | 将所有来自 hub 的工具重新下载为最新版本 |

如需向 hub 贡献工具，请在 [garudust-org/garudust-hub](https://github.com/garudust-org/garudust-hub) 提交 PR。

---

## 架构

```
  garudust (CLI)              garudust-server
  ┌────────────────────┐    ┌─────────────────────────────────────────────┐
  │  TUI / one-shot    │    │  HTTP /chat · /stream · /ws                 │
  │  setup · config    ├──┐ │  Telegram · Discord · Slack · Matrix · LINE · WhatsApp │
  │  doctor · model    │  │ │  Webhook · Cron                             │
  └────────────────────┘  │ └──────────────────────────┬──────────────────┘
                          │                            │
                          └─────────────┬──────────────┘
                                        ▼
                               ┌─────────────────┐
                               │      Agent       │
                               │   run_loop()     │
                               └────────┬─────────┘
                            ┌───────────┴───────────┐
                            ▼                       ▼
              ┌──────────────────────┐  ┌─────────────────────────────────┐
              │      Transport       │  │        ToolRegistry              │
              │  Anthropic           │  │  web_fetch · web_search          │
              │  OpenRouter          │  │  http_request · browser          │
              │  AWS Bedrock         │  │  read_file · write_file          │
              │  Codex               │  │  list_directory · terminal       │
              │  Ollama · vLLM       │  │  memory · user_profile           │
              └──────────────────────┘  │  session_search · delegate_task  │
                                        │  skills · + MCP (external)       │
                                        └─────────────┬───────────────────┘
                                                      │
                                          ┌───────────┴───────────┐
                                          ▼                       ▼
                                ┌──────────────────┐  ┌──────────────────────┐
                                │ FileMemoryStore   │  │      SessionDb       │
                                │ memory/ · skills/ │  │   SQLite + FTS5      │
                                └──────────────────┘  └──────────────────────┘
```

### Crate 布局

| Crate / 二进制 | 职责 |
|---|---|
| `garudust-core` | 共享 trait 和类型 — 零 I/O |
| `garudust-transport` | LLM 适配器：Anthropic、OpenAI-compat、Bedrock、Codex、Ollama、vLLM |
| `garudust-tools` | 工具注册表 + 内置工具集（web、files、terminal、browser 等） |
| `garudust-memory` | `FileMemoryStore`（markdown）+ `SessionDb`（SQLite + FTS5） |
| `garudust-agent` | Agent 运行循环、上下文压缩器、提示构建器 |
| `garudust-platforms` | Telegram、Discord、Slack、Matrix、LINE、WhatsApp、Webhook |
| `garudust-cron` | 定时调度器 |
| `garudust-gateway` | axum HTTP 网关 — `/chat`、`/chat/stream`、`/chat/ws`、`/metrics` |
| `bin/garudust` | CLI：交互式 TUI、单次任务、`setup`、`config`、`doctor`、`model` |
| `bin/garudust-server` | 无头模式：所有平台 + HTTP 网关 + 定时任务，单进程运行 |

---

## 贡献

Garudust 设计为易于扩展 — 添加工具、传输层或平台适配器通常只需修改一个 crate，代码不超过 100 行。

### 新手入门议题

- **新工具** — 在 `garudust-tools` 中将任意 CLI 或 API 封装为 `Tool` 实现
- **新平台** — 实现 `PlatformAdapter`（如 Signal、WeChat）
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

有问题或发现 Bug？加入 [Discord 社区](https://discord.com/channels/1501414298449088745/1501414298893942877) 交流，或提交 [GitHub issue](https://github.com/garudust-org/garudust-agent/issues)。

---

## 许可证

MIT — 详见 [LICENSE](../../../LICENSE)

---

## 贡献者

[![](https://contrib.rocks/image?repo=garudust-org/garudust-agent)](https://github.com/garudust-org/garudust-agent/graphs/contributors)

---

## Star 历史

[![Star History Chart](https://api.star-history.com/svg?repos=garudust-org/garudust-agent&type=Date)](https://star-history.com/#garudust-org/garudust-agent&Date)

---

<div align="center">
  <img src="https://visitor-badge.laobi.icu/badge?page_id=garudust-org.garudust-agent&style=flat" alt="visitors"/>
</div>
