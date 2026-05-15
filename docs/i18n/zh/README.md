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
- **一个环境变量切换提供商** — 支持 Anthropic、OpenRouter、AWS Bedrock、Ollama、vLLM、ThaiLLM
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

```bash
garudust setup   # 首次配置向导 — 选择提供商、保存 API 密钥
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
| `↑ ↓` | 滚动历史 |
| `/new` | 开始新会话 |
| `/model <name>` | 即时切换模型 |
| `Ctrl+C` | 退出 |

### 2 — 单次执行

```bash
garudust "将最近 7 天的 git log 整理成 changelog"
```

输出至 stdout，成功时退出码为 0，支持管道操作。

### 3 — 服务器模式

```bash
garudust-server --port 3000
```

开放 `POST /chat`、`POST /chat/stream` 和 `ws://…/chat/ws`。在 `~/.garudust/.env` 中设置对应 token，并在 `~/.garudust/config.yaml` 中启用相应平台：

```bash
# ~/.garudust/.env  — 仅存放密钥，不要提交到版本控制
ANTHROPIC_API_KEY=sk-ant-...
TELEGRAM_TOKEN=123456789:AAFxxx
LINE_CHANNEL_TOKEN=<channel-access-token>
LINE_CHANNEL_SECRET=<32位十六进制密钥>
DISCORD_TOKEN=<bot-token>
BRAVE_SEARCH_API_KEY=BSA...        # 可选 — 未设置时回退到 DuckDuckGo
```

```yaml
# ~/.garudust/config.yaml
model: anthropic/claude-sonnet-4-6
provider: anthropic

platforms:
  telegram:
    enabled: true
  discord:
    enabled: true
  line:
    enabled: true
    port: 3002
    webhook_path: /line    # LINE 控制台 webhook → https://your-host:3002/line

security:
  terminal_sandbox: docker           # 在隔离容器中运行 shell 命令
  approval_mode: smart               # smart | auto | deny

cron:
  memory_consolidation: "0 3 * * *" # 每晚自动整理记忆
```

```bash
# 快速 API 测试
curl -X POST http://localhost:3000/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "写一首关于 Rust 的俳句"}'

# 流式输出（Server-Sent Events）
curl -X POST http://localhost:3000/chat/stream \
  -H "Content-Type: application/json" \
  -d '{"message": "用 3 句话解释 async/await"}'
```

### 4 — Docker

```bash
# 1. 创建 .env 文件存放密钥
cat > .env <<'EOF'
ANTHROPIC_API_KEY=sk-ant-...
TELEGRAM_TOKEN=123456789:AAFxxx        # 可选 — 不用则删除
LINE_CHANNEL_TOKEN=<token>             # 可选
LINE_CHANNEL_SECRET=<secret>           # 可选
GARUDUST_API_KEY=my-gateway-secret     # 保护 HTTP API
GARUDUST_APPROVAL_MODE=smart
EOF

# 2. 启动
docker compose up -d

# 3. 检查健康状态
curl http://localhost:3000/health
```

数据持久化在 Docker volume `garudust-data`（容器内为 `/root/.garudust`）。如需使用自定义 `config.yaml`，可通过 bind-mount 挂载：

```yaml
# docker-compose.yml（覆盖配置）
services:
  garudust:
    volumes:
      - garudust-data:/root/.garudust
      - ./config.yaml:/root/.garudust/config.yaml:ro
```

<div align="center">
  <img src="../../../assets/demo-line.jpg" alt="LINE Demo" width="420"/>
</div>

---

## v0.4.0 新特性

| 特性 | 说明 |
|---|---|
| 并行工具执行 | 基于 `parallelism_key` 分组 — 独立工具并发，冲突写入自动串行 |
| 凭证轮换 | `LLM_FALLBACK_API_KEYS=key2,key3` — 鉴权失败时自动轮换，无需重启 |
| 三区域上下文压缩 | 保留原始任务（头部）+ 最新轮次（尾部），仅摘要中间部分 |
| `AgentHooks` trait | `on_turn_start`、`on_session_end`、`on_pre_compress`、`on_delegation` |
| 扩展推理强度 | `Minimal`（512 tokens）→ `Low` → `Medium` → `High` → `XHigh`（32k tokens） |
| 子智能体迭代预算 | `sub_agent_max_iterations` 独立于主智能体单独限制委派链深度 |
| FTS5 trigram 搜索 | 子串会话搜索 — `"pythag"` 可匹配 `"Pythagorean"`，包含自动 DB 迁移 |
| WAL 模式降级 | 在 NFS/SMB 文件系统上优雅降级，而非崩溃 |

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
| OpenRouter | `provider: openrouter` *（默认）* | `OPENROUTER_API_KEY` |
| AWS Bedrock | `provider: bedrock` | `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` |
| Ollama | `provider: ollama` + `base_url` | *（无需）* |
| vLLM | `provider: vllm` + `base_url` | `VLLM_API_KEY` |
| ThaiLLM | `provider: thaillm` | `THAILLM_API_KEY` |
| 任意 OpenAI 兼容 | `provider: custom` + `base_url` | 对应 API 密钥 |

备用密钥：`LLM_FALLBACK_API_KEYS=key2,key3` — 鉴权失败时自动轮换

---

## 技能与记忆

智能体将所学内容保存至 `~/.garudust/memory/`，并在每次会话开始时自动加载。可复用的工作流会被自动写入 `~/.garudust/skills/` 作为技能，无需手动操作。

从 [agentskills.io](https://agentskills.io) 安装技能：

```bash
garudust skill install agentskills-org/hub/git-workflow
garudust tool install weather   # 社区脚本工具
```

---

## 参与贡献

添加工具、传输层或平台适配器通常只需修改一个 crate，代码量不超过 100 行。详见 [CONTRIBUTING.md](../../../CONTRIBUTING.md)。

发现问题或有疑问？[提交 issue](https://github.com/garudust-org/garudust-agent/issues) 或加入 [Discord 社区](https://discord.com/channels/1501414298449088745/1501414298893942877)。

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
