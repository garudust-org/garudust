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
[![Discord](https://img.shields.io/badge/Discord-ชุมชน-5865F2?logo=discord&logoColor=white&style=flat-square)](https://discord.com/channels/1501414298449088745/1501414298893942877)

**AI agent ของคุณ เซิร์ฟเวอร์ของคุณ กฎของคุณ**

AI agent runtime แบบ self-improving เขียนด้วย Rust — binary เดียวขนาด ~10 MB ไม่มี runtime dependency แชทในเทอร์มินัล ตอบบน 7 แพลตฟอร์ม หรือเปิด REST + WebSocket API เชื่อมต่อ MCP server ใดก็ได้ ให้ agent เขียน skill เองและเปลี่ยน LLM ด้วย env var เดียว ไม่มี telemetry ไม่ผูกติด vendor

<div align="center">
  <img src="../../../assets/demo.svg" alt="Garudust demo"/>
</div>

---

## สารบัญ

- [ทำไมต้อง Garudust?](#ทำไมต้อง-garudust)
- [Supported Platforms](#supported-platforms)
- [เริ่มใช้งาน](#เริ่มใช้งาน)
- [สถาปัตยกรรม](#สถาปัตยกรรม)
- [การตั้งค่า](#การตั้งค่า)
- [แพลตฟอร์ม](#แพลตฟอร์ม)
- [LLM Provider](#llm-provider)
- [Tools](#tools)
- [RAG (ค้นหาในเอกสาร)](#rag-ค้นหาในเอกสาร)
- [Hub](#hub)
- [Memory](#memory)
- [การควบคุมการเข้าถึง](#การควบคุมการเข้าถึง)
- [หมายเหตุด้านความปลอดภัย](#หมายเหตุด้านความปลอดภัย)
- [ร่วมพัฒนา](#ร่วมพัฒนา)
- [License](#license)
- [ผู้ร่วมพัฒนา](#ผู้ร่วมพัฒนา)

---

## ทำไมต้อง Garudust?

- **~10 MB, cold start < 20 ms** — ไฟล์เดียว ไม่ต้องพึ่ง runtime อื่น
- **พัฒนาตัวเองได้** — จดจำความชอบของคุณ สร้าง skill จาก workflow และแก้ไขตัวเองโดยไม่ต้องบอกซ้ำ
- **รัน tool พร้อมกัน** — จัดกลุ่มตาม `parallelism_key` ทำงานคู่ขนานโดยอัตโนมัติ serializes เฉพาะที่จำเป็น
- **หมุนเวียน API key อัตโนมัติ** — ตั้ง `LLM_FALLBACK_API_KEYS` แล้ว agent จะสลับ key เมื่อเจอ auth error โดยไม่หยุดทำงาน
- **บีบอัด context อัจฉริยะ** — แบ่ง 3 zone: เก็บ task เดิมและ turn ล่าสุดไว้ สรุปเฉพาะตรงกลาง
- **Lifecycle hooks** — `AgentHooks` callback ทุก turn, compression, delegation และ session end
- **รองรับ agentskills.io** — ติดตั้ง skill จาก hub หรือ GitHub repo ใดก็ได้ด้วยคำสั่งเดียว
- **7 แพลตฟอร์มในกระบวนการเดียว** — Telegram, Discord, Slack, Matrix, LINE, WhatsApp, Webhook
- **24 LLM provider, กำหนดเป็น named profile** — รองรับ Anthropic, OpenAI, Gemini, Groq, Mistral, DeepSeek, xAI, Together AI, Fireworks, Cerebras, Perplexity, Cohere, NVIDIA NIM, Alibaba DashScope, ByteDance Doubao, Zhipu AI, Moonshot, Baidu ERNIE, OpenRouter, AWS Bedrock, Ollama, vLLM, ThaiLLM — กำหนด `providers:` profile ใน `config.yaml` และ route ต่อ task ได้
- **Provider routing hints** — กำหนด hint name → provider/model ใน config แล้วส่ง `--hint fast` เพื่อเปลี่ยน model เฉพาะ task นั้นโดยไม่กระทบ default
- **กำหนด model ต่อ tool** — override model (และ fallback) ที่แต่ละ hub tool ใช้ผ่าน `tools.<name>.model` ใน `config.yaml`
- **ปลอดภัยตั้งแต่ต้น** — Docker sandbox, บล็อคคำสั่งอันตราย, ป้องกัน memory poisoning, redact secret อัตโนมัติ

---

## Supported Platforms

ดาวน์โหลด binary สำเร็จรูปจาก [**GitHub Releases**](https://github.com/garudust-org/garudust-agent/releases/latest) — ไม่ต้องติดตั้ง Rust:

| แพลตฟอร์ม | ไฟล์ |
|-----------|------|
| macOS Apple Silicon | `garudust-*-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `garudust-*-x86_64-apple-darwin.tar.gz` |
| Linux x86_64 | `garudust-*-x86_64-unknown-linux-musl.tar.gz` |
| Linux ARM64 (Raspberry Pi 4/5, Jetson) | `garudust-*-aarch64-unknown-linux-musl.tar.gz` |
| Windows | `garudust-*-x86_64-pc-windows-msvc.zip` |

---

## เริ่มใช้งาน

**01 — ติดตั้ง**

ดาวน์โหลด binary จาก [GitHub Releases](https://github.com/garudust-org/garudust-agent/releases/latest):

```bash
# ตรวจ architecture อัตโนมัติ (x86_64 หรือ ARM64 — Raspberry Pi 4/5, Jetson)
ARCH=$(uname -m)
[ "$ARCH" = "aarch64" ] && TARGET="aarch64-unknown-linux-musl" || TARGET="x86_64-unknown-linux-musl"
VER=$(curl -s https://api.github.com/repos/garudust-org/garudust-agent/releases/latest | grep tag_name | cut -d'"' -f4)
curl -LO "https://github.com/garudust-org/garudust-agent/releases/latest/download/garudust-${VER}-${TARGET}.tar.gz"
tar -xzf garudust-*.tar.gz
sudo mv garudust garudust-server /usr/local/bin/
```

หรือ build จาก source (Rust 1.87+):

```bash
git clone https://github.com/garudust-org/garudust-agent
cd garudust-agent && cargo build --release
```

---

**02 — ตั้งค่า**

รัน wizard ครั้งแรก — เลือก provider ใส่ API key แล้วเขียน `~/.garudust/config.yaml` (พร้อม `providers.default` profile) และ `~/.garudust/.env` ให้อัตโนมัติ **โหมด Full** ยังตั้งค่า platform adapters และมีตัวเลือก [access control แบบ invite-only](#การควบคุมการเข้าถึง) ที่ seed role `admin` + `member` ให้อัตโนมัติ:

```bash
garudust setup
```

หรือใส่ key ตรงใน `~/.garudust/.env` เอง ดู [การตั้งค่า](#การตั้งค่า) สำหรับ `config.yaml` แบบเต็ม

---

**03 — รัน**

```bash
garudust                                  # interactive TUI
garudust "สรุป git log"                   # one-shot task
garudust --hint fast "ตรวจสอบโค้ดนี้"   # ใช้ model ที่ถูกกว่า
garudust-server --port 3000               # headless server (REST + WS)
docker compose up -d                      # Docker
```

---

### ปุ่มลัด TUI

<div align="center">
  <img src="../../../assets/demo-tui.png" alt="Garudust TUI" width="800"/>
</div>

| ปุ่ม | การทำงาน |
|------|----------|
| `Enter` | ส่งข้อความ |
| `↑ ↓` | เลื่อนประวัติ |
| `/new` | เริ่ม session ใหม่ |
| `/model <name>` | เปลี่ยน model ทันที |
| `Ctrl+C` | ออกจากโปรแกรม |

---

## สถาปัตยกรรม

```
┌──────────────────────────────────────────────────────────────────────┐
│  bin/garudust (CLI)              bin/garudust-server (Daemon)        │
│  garudust [task] [--hint H]      garudust-server --port 3000         │
└────────────────────┬─────────────────────────┬───────────────────────┘
                     │                         │
                     │          ┌──────────────┴───────────────────────┐
                     │          │  garudust-gateway  (server-only)     │
                     │          │  POST /chat · POST /stream · GET /ws │
                     │          │  RBAC · roles · /join · /invite      │
                     │          │  SessionRegistry · Metrics           │
                     │          ├──────────────────────────────────────┤
                     │          │  garudust-platforms  (server-only)   │
                     │          │  Telegram · Discord · Slack          │
                     │          │  LINE · Matrix · WhatsApp · Webhook  │
                     │          ├──────────────────────────────────────┤
                     │          │  garudust-cron  (server-only)        │
                     │          │  task อัตโนมัติตาม cron schedule    │
                     │          └──────────────┬───────────────────────┘
                     │                         │
                     ▼                         ▼
┌──────────────────────────────────────────────────────────────────────┐
│                    garudust-agent  (run-loop)                        │
│                                                                      │
│  1. โหลด memory.md + user_profile.md                                │
│  2. สร้าง system prompt — แทรก skills note                          │
│  3. Resolve routing hint → transport + model                        │
│                                                                      │
│  LOOP (max_iterations = 90):                                         │
│    a. เรียก LLM (streaming) → ได้ text + tool_calls                 │
│    b. Validate schema → ตรวจ permission → approval gate             │
│    c. Execute tools (parallel เมื่อปลอดภัย, มี timeout)             │
│    d. ห่อ untrusted output → เพิ่มผลลัพธ์เข้า history              │
│    e. stop_reason == EndTurn → หยุด loop                            │
│                                                                      │
│  4. บันทึกบทสนทนา → ~/.garudust/conversations/{hash}.json           │
│  5. บันทึก log → SessionDb (SQLite)                                 │
└──────┬──────────────┬─────────────────┬─────────────────────────────┘
       │              │                 │
       ▼              ▼                 ▼
┌────────────┐ ┌────────────┐ ┌──────────────┐
│ garudust-  │ │ garudust-  │ │  garudust-   │
│ transport  │ │ tools      │ │  memory      │
│            │ │            │ │              │
│ 24 LLM     │ │ Built-in   │ │ memory.md    │
│ provider   │ │ Hub/Script │ │ user_profile │
│ Named      │ │ MCP        │ │ sessions.db  │
│ profiles   │ │            │ │ docs.db(RAG) │
│ Retry +    │ │            │ │              │
│ rotation   │ │            │ │              │
└────────────┘ └────────────┘ └──────────────┘

garudust-core — shared types · config · traits · pricing (ใช้โดยทุก crate ข้างต้น)
```

**Gateway** — `garudust-gateway` expose agent ผ่าน HTTP/WebSocket/SSE ด้วย axum router (`/chat`, `/stream`, `/ws`, `/health`, `/metrics`) `GatewayHandler` คั่นกลางระหว่าง platform กับ agent: บังคับใช้ RBAC, resolve approver ต่อ user, จัดการ session และรับคำสั่ง runtime (`/join`, `/invite`, `/role`, `/whoami`) ใช้เฉพาะ server binary

**Platforms** — `garudust-platforms` ให้ implementation ของ `PlatformAdapter` สำหรับแต่ละ messaging service (Telegram, Discord, Slack, LINE, Matrix, WhatsApp, Webhook) ใช้เฉพาะ server binary เท่านั้น CLI ไม่มี platform layer

**Cron** — `garudust-cron` รัน agent task ตาม cron schedule (รูปแบบ `"0 0 9 * * *=morning briefing"`) ใช้ 6-field cron syntax (sec min hour dom month dow) ใช้โดย server binary สำหรับ agent run แบบอัตโนมัติ ไม่ต้องมีคนคุย

**Transport** — `garudust-transport` resolve `providers.default` (หรือ named profile) ไปเป็น API client ที่เหมาะสม: native Anthropic SDK, OpenAI-compatible HTTP, Bedrock หรือ Ollama พร้อม retry แบบ exponential backoff และ credential rotation อัตโนมัติ

**Tools** — 3 ประเภท: *built-in* (files, terminal, browser, web, memory, git, rag, delegate, cron, notes), *hub/script* (ดาวน์โหลดไปที่ `~/.garudust/tools/` ใช้ภาษาใดก็ได้) และ *MCP* (Model Context Protocol server ใดก็ได้) ทุกตัวผ่านเส้นทางเดียวกัน: validate schema → ตรวจ permission → approval gate → รันพร้อม timeout

**Memory** — `FileMemoryStore` เขียน `memory.md` และ `user_profile.md` (Markdown); `SessionDb` บันทึกประวัติการสนทนาและ tool-call log ใน SQLite; `DocStore` ทำ FTS5 full-text search สำหรับ RAG

**Skills** — ไฟล์ Markdown (`~/.garudust/skills/*.md`) แทรกเป็น hint ใน system prompt `skill_view` โหลดเนื้อหา skill ทั้งหมดและบังคับใช้ `required_tools` และ `permissions` ที่ประกาศไว้ในรอบนั้น skill ที่ใช้ซ้ำได้จะถูกสร้างอัตโนมัติหลัง `auto_skill_threshold` iterations

**Routing** — `--hint <name>` map ไปยัง `routing:` entry ใน `config.yaml` (รูปแบบ `"profile/model"` หรือ `"provider/model"`) เปลี่ยน transport และ model เฉพาะ task นั้นโดยไม่กระทบ default

---

## การตั้งค่า

Secret เก็บใน `~/.garudust/.env` ส่วนการตั้งค่าอื่น ๆ อยู่ใน `~/.garudust/config.yaml` รัน `garudust setup` เพื่อสร้างทั้งสองไฟล์แบบ interactive

### `~/.garudust/.env`

```bash
# LLM provider — ตั้ง 1 ตัว (ถ้าไม่มี config.yaml จะ detect อัตโนมัติจาก env)
ANTHROPIC_API_KEY=sk-ant-...
# OPENAI_API_KEY=sk-...
# GEMINI_API_KEY=AIza...
# GROQ_API_KEY=gsk_...
# MISTRAL_API_KEY=...
# DEEPSEEK_API_KEY=sk-...
# XAI_API_KEY=xai-...
# OPENROUTER_API_KEY=sk-or-...
# VLLM_API_KEY=...

# Fallback keys — สลับอัตโนมัติเมื่อ auth ล้มเหลว
# LLM_FALLBACK_API_KEYS=sk-ant-backup1,sk-ant-backup2

# Platform adapters — ตั้งเฉพาะที่ใช้
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
BRAVE_SEARCH_API_KEY=BSA...      # optional — fallback เป็น DuckDuckGo
SERPER_API_KEY=...               # optional — ค้นหาผ่าน Google (Serper)

# ป้องกัน HTTP API
GARUDUST_API_KEY=my-gateway-secret
```

### `~/.garudust/config.yaml`

```yaml
# ── Provider profiles ─────────────────────────────────────────────────────────
# providers.default กำหนด LLM หลัก เก็บ API key ใน ~/.garudust/.env
providers:
  default:
    name: anthropic          # anthropic | openai | gemini | groq | mistral | deepseek
                             # xai | openrouter | ollama | vllm | thaillm | bedrock
                             # together | fireworks | cerebras | perplexity | cohere
                             # nvidia | alibaba | doubao | zhipu | moonshot | baidu
    key: ${ANTHROPIC_API_KEY}
    model: claude-sonnet-4-6

  # Named profile เพิ่มเติมสำหรับ routing หรือ tool model override:
  # groq-fast:
  #   name: groq
  #   key: ${GROQ_API_KEY}
  #   model: llama-3.1-8b-instant
  #
  # local:
  #   url: http://localhost:11434/v1   # OpenAI-compatible endpoint แบบกำหนดเอง
  #   model: llama3.2

# ── Agent settings ────────────────────────────────────────────────────────────
max_iterations: 90
max_output_tokens: 8192
context_window: 128000      # ลดลงสำหรับ model ที่ context เล็ก เช่น 32768
reasoning_effort: ~         # low | medium | high  (Claude extended thinking / OpenAI o-series)
show_usage_footer: false

# ── Timeout & retry ──────────────────────────────────────────────────────────
llm_timeout_secs: 120
tool_timeout_secs: 60
llm_max_retries: 3

# ── Provider routing hints (เปลี่ยน model ต่อ task) ─────────────────────────
# ส่ง --hint <name> ที่ CLI หรือ hint: "name" ใน API payload
# รูปแบบ: "profile/model" (ใช้ named profile) หรือ "provider/model" (builtin)
routing:
  fast:   groq-fast/llama-3.1-8b-instant   # ใช้ groq-fast profile ที่กำหนดไว้
  vision: openai/gpt-4o                     # builtin provider name
  smart:  anthropic/claude-opus-4-7

# ── กำหนด model ต่อ tool ─────────────────────────────────────────────────────
# ส่งเป็น GARUDUST_MODEL / GARUDUST_FALLBACK_MODEL ให้ subprocess ของ tool
# tool ที่ไม่อ่าน env var นี้ไม่ได้รับผลกระทบ (backward compat เต็ม)
tools:
  view_image:
    model: openrouter/google/gemini-flash-1.5
    fallback_model: google/gemini-1.5-flash

# ── ปิด tool / toolset ───────────────────────────────────────────────────────
# disabled_toolsets: [browser, git, notes]
# disabled_tools: [image_read, pdf_read]

# ── Security ──────────────────────────────────────────────────────────────────
security:
  approval_mode: smart        # auto | smart | deny
                              # smart = ตรวจสอบ tool ที่มีความเสี่ยงแต่ไม่บล็อก
                              # ใช้ deny เพื่อบล็อกทุก tool call ที่ไม่ได้รับอนุญาต
  terminal_sandbox: none      # none | docker
                              # คำเตือน: none รัน shell command บน host โดยตรง
                              # ใช้ docker ใน production เพื่อ isolate การรัน command
  rate_limit_rpm: ~           # จำกัด request ต่อ IP ต่อนาที (~ = ไม่จำกัด)
  allowed_read_paths: []      # default: cwd + home
  allowed_write_paths: []     # default: cwd

# ── Sub-agent delegation ──────────────────────────────────────────────────────
# max_delegation_depth: 1     # ความลึกสูงสุดของการ delegate ซ้อนกัน (default 1)
                              # 0 = sub-agent ไม่สามารถ delegate ต่อได้
                              # ป้องกันการ delegate ซ้อนกันไม่สิ้นสุด

# ── Memory expiry ─────────────────────────────────────────────────────────────
memory_expiry:
  fact_days: 90               # null = ไม่หมดอายุ
  project_days: 30
  other_days: 60
  preference_days: ~
nudge_interval: 5             # เตือนบันทึก memory ทุก N tool rounds (0 = ปิด)
auto_skill_threshold: 5       # เขียน skill อัตโนมัติหลัง N iterations (0 = ปิด)

# ── Platform / กลุ่มแชท ──────────────────────────────────────────────────────
platform:
  require_mention: false      # true = ตอบเฉพาะเมื่อถูก @mention ในกลุ่ม
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

# ── Cron jobs ─────────────────────────────────────────────────────────────────
cron:
  # 6-field syntax: sec min hour dom month dow
  memory_consolidation: "0 0 3 * * *"   # housekeeping memory ทุกคืน
  memory_expiry: "0 0 4 * * 0"          # ลบ memory ที่หมดอายุรายสัปดาห์
  jobs:
    - schedule: "0 0 9 * * 1-5"
      task: "เขียน morning briefing และบันทึกที่ ~/briefing.md"

# ── Context compression ───────────────────────────────────────────────────────
compression:
  enabled: true
  threshold_fraction: 0.8     # บีบเมื่อใช้ context ครบ 80%
  model: ~                    # model แยกสำหรับ compression (default: model หลัก)

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

## แพลตฟอร์ม

<div align="center">
  <a href="https://core.telegram.org/bots"><img src="https://img.shields.io/badge/Telegram-2CA5E0?logo=telegram&logoColor=white&style=for-the-badge"/></a>
  <a href="https://discord.com/developers/applications"><img src="https://img.shields.io/badge/Discord-5865F2?logo=discord&logoColor=white&style=for-the-badge"/></a>
  <a href="https://api.slack.com/apps"><img src="https://img.shields.io/badge/Slack-4A154B?logo=slack&logoColor=white&style=for-the-badge"/></a>
  <a href="https://matrix.org"><img src="https://img.shields.io/badge/Matrix-000000?logo=matrix&logoColor=white&style=for-the-badge"/></a>
  <a href="https://developers.line.biz/console/"><img src="https://img.shields.io/badge/LINE-00C300?logo=line&logoColor=white&style=for-the-badge"/></a>
  <a href="https://developers.facebook.com/docs/whatsapp/cloud-api"><img src="https://img.shields.io/badge/WhatsApp-25D366?logo=whatsapp&logoColor=white&style=for-the-badge"/></a>
  <img src="https://img.shields.io/badge/Webhook-6E7681?style=for-the-badge"/>
</div>

ทุก adapter รันในกระบวนการเดียวกันกับ `garudust-server` ตั้ง token ใน `~/.garudust/.env` ก็พร้อมใช้งานทันที

---

## LLM Provider

ตั้งค่า `providers.default.name` ใน `config.yaml` และ key ที่เกี่ยวข้องใน `~/.garudust/.env`:

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
| Ollama | `ollama` *(เพิ่ม `url:` สำหรับ endpoint กำหนดเอง)* | *(ไม่ต้องการ)* |
| vLLM | `vllm` *(เพิ่ม `url:` สำหรับ endpoint กำหนดเอง)* | `VLLM_API_KEY` |
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
| OpenAI-compat อื่น ๆ | *(ไม่ต้องใส่ `name:` ตั้งค่า `url:` ใน profile แทน)* | API key ที่เกี่ยวข้อง |

Fallback keys: `LLM_FALLBACK_API_KEYS=key2,key3` — สลับอัตโนมัติเมื่อ auth ล้มเหลว

---

## Tools

built-in tools พร้อมใช้ทันที ไม่ต้องตั้งค่าเพิ่ม

| Tool | คำอธิบาย |
|------|----------|
| `web_fetch` | ดึงข้อมูลจาก URL |
| `web_search` | ค้นหาเว็บ (Brave / Serper / DuckDuckGo) |
| `http_request` | HTTP request แบบกำหนดเอง พร้อม headers และ body |
| `browser` | ควบคุม Chrome/Chromium ผ่าน CDP — คลิก, พิมพ์, screenshot, รัน JS |
| `read_file` / `write_file` | อ่านและเขียนไฟล์ |
| `list_directory` | แสดงไฟล์ด้วย glob pattern และ depth limit |
| `terminal` | รัน shell command (รองรับ Docker sandbox — ดูหมายเหตุด้านความปลอดภัย) |
| `memory` | memory แบบ key-value ที่คงอยู่ข้าม session |
| `session_search` | ค้นหาประวัติการสนทนา (FTS5 trigram) |
| `delegate_task` | spawn sub-agent แบบ parallel สำหรับงานย่อย (จำกัดความลึกด้วย `max_delegation_depth`) |
| `skill_view` / `write_skill` | โหลดและเขียน skill ที่ใช้ซ้ำได้ |
| `doc_ingest` | นำเข้าเอกสาร (PDF, TXT, CSV, MD, …) เข้าสู่ดัชนีค้นหา |
| `doc_search` | ค้นหาข้อความในเอกสารที่นำเข้าทั้งหมด |
| `doc_list` | แสดงรายการเอกสารที่นำเข้าใน session ปัจจุบัน |
| `doc_forget` | ลบเอกสารหนึ่งหรือทั้งหมดออกจาก RAG index |

**Custom script tools** — วาง `tool.yaml` + script ใน `~/.garudust/tools/<name>/`:

```yaml
# tool.yaml
name: get_weather
description: ดูสภาพอากาศของเมือง
schema:
  type: object
  properties:
    city: { type: string }
  required: [city]
command: "curl -s wttr.in/{city}?format=3"
# env_required: [MY_API_KEY]   # ส่ง secret เฉพาะที่ระบุจาก ~/.garudust/.env
```

กำหนด model สำหรับแต่ละ tool ใน `config.yaml` ทั้ง `model` และ `fallback_model` รับรูปแบบ `"profile/model"` (named profile จาก `providers:`) หรือ `"provider/model"` (builtin) subprocess จะได้รับ `GARUDUST_MODEL` / `GARUDUST_BASE_URL` / `GARUDUST_API_KEY` สำหรับ primary และ `GARUDUST_FALLBACK_MODEL` / `GARUDUST_FALLBACK_BASE_URL` / `GARUDUST_FALLBACK_API_KEY` สำหรับ fallback:

```yaml
tools:
  get_weather:
    model: groq-fast/llama-3.1-8b-instant        # "groq-fast" = named profile
    fallback_model: openrouter/meta-llama/llama-3.1-8b-instruct  # builtin provider
  view_image:
    model: vision/gemini-flash-latest             # "vision" = named profile (เช่น gemini key)
    fallback_model: vision-fallback/nvidia/nemotron-nano-12b-v2-vl:free
```

**MCP** — เชื่อมต่อ [Model Context Protocol](https://modelcontextprotocol.io) server ใดก็ได้ใน `config.yaml`:

```yaml
mcp_servers:
  - name: filesystem
    command: npx
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
```

---

## RAG (ค้นหาในเอกสาร)

นำเข้าเอกสารแล้วถามคำถามได้เลย — agent ค้นหาให้อัตโนมัติเมื่อคำถามเกี่ยวข้อง

**รูปแบบที่รองรับ:** PDF, TXT, CSV, MD, JSON, DOCX, DOC, XLSX, XLS

**ผ่านแชทแพลตฟอร์ม** — ส่งไฟล์ ยืนยันเมื่อบอทถาม แล้วถามคำถามได้ทันที

**ผ่าน CLI** — พิมพ์: `นำเข้าไฟล์ /home/user/report.pdf`

agent เรียก `doc_search` ให้เอง ใช้ `doc_list` ดูรายการไฟล์ และ `doc_forget` เพื่อลบ แต่ละ session มี index แยกกันโดยอิสระ

ปิดใช้งานได้ใน `config.yaml`:

```yaml
disabled_toolsets: ["rag"]
```

---

## Hub

คำสั่งเดียวเพื่อขยายความสามารถของ agent ด้วย tool และ skill จากชุมชน

**Tool Hub** ([garudust-hub](https://github.com/garudust-org/garudust-hub))

```bash
garudust tool list                        # ดู tool ที่มี
garudust tool install weather             # ติดตั้งไปที่ ~/.garudust/tools/weather/
garudust tool install hash_text
garudust tool uninstall weather
garudust tool update                      # อัปเดต hub tools ทั้งหมด
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

agent บันทึกทุกสิ่งที่เรียนรู้ไว้ใน `~/.garudust/memory/` และโหลดทุก session — ไม่ต้องบอกซ้ำ workflow ที่ใช้ซ้ำได้จะถูกเขียนเป็น skill ใน `~/.garudust/skills/` โดยอัตโนมัติ

```
คุณ: format JSON ด้วย 2-space indent เสมอ
Agent: รับทราบ — บันทึกไว้ใน memory แล้ว
# session ถัดไป: ทำให้เลย ไม่ต้องเตือนอีก
```

---

## การควบคุมการเข้าถึง

Garudust รองรับ Role-Based Access Control (RBAC) ผ่าน `roles:` ใน `config.yaml`
แต่ละผู้ใช้จะถูกกำหนด role โดย role จะกำหนดว่าสามารถใช้ tool ใดได้บ้าง และ tool เหล่านั้นต้องผ่านการอนุมัติอย่างไร

### นิยาม Role

| ฟิลด์ | ค่าที่รองรับ | คำอธิบาย |
|---|---|---|
| `approval_mode` | `auto` / `smart` / `deny` | วิธีอนุมัติการเรียก tool สำหรับ role นี้ |
| `allowed_toolsets` | รายชื่อ toolset | จำกัดให้ใช้ได้เฉพาะ toolset เหล่านี้ (ว่าง = ไม่จำกัด) |
| `allowed_tools` | รายชื่อ tool | tool เพิ่มเติมที่อนุญาตรายตัว |
| `denied_tools` | รายชื่อ tool | tool ที่บล็อกเสมอ ไม่ว่า toolset จะกำหนดอะไรก็ตาม |

ค่าของ `approval_mode`:

- **`auto`** — อนุมัติทุก tool call ทันที
- **`smart`** — ตรวจสอบด้วย constitutional policy (พฤติกรรมเริ่มต้นระดับ global)
- **`deny`** — บล็อก tool call ทั้งหมด; bot ยังคงตอบข้อความได้ปกติ

### การกำหนด User

User ถูก key ด้วย platform และ ID ใต้ `roles.users`
Telegram รองรับทั้ง numeric ID และ `@username`
User ที่ไม่พบในแผนที่จะใช้ `default_role`; ถ้าไม่มี `default_role` ระบบจะใช้ `security.approval_mode` ระดับ global

### Bootstrap

เมื่อ `roles:` ถูกตั้งค่าพร้อม role definition ของ `admin` แต่ยังไม่มี user ถูกกำหนด role ใดเลย
**คนแรกที่ส่ง DM** จะได้รับการเลื่อนตำแหน่งเป็น `admin` อัตโนมัติ
ช่วยให้ deploy ด้วย `roles.users` ว่างเปล่าและอ้างสิทธิ์ ownership ได้ตั้งแต่การติดต่อครั้งแรก
ไม่ต้องค้นหา platform ID และแก้ `config.yaml` ด้วยตนเองล่วงหน้า

### การ Gate ผู้ใช้ที่ไม่รู้จัก

เมื่อ roles ทำงานอยู่และ user ที่เข้ามาไม่มี role และไม่มี `default_role` ที่ใช้ได้
agent จะไม่เพิกเฉยต่อพวกเขา แต่จะตอบกลับพร้อมคำแนะนำว่าการเข้าถึงถูกจำกัด
และสั่งให้พิมพ์ `/join` เพื่อขอสิทธิ์เข้าถึง

### ขั้นตอนการลงทะเบียนด้วยตนเอง

มีสองวิธีที่ผู้ใช้ใหม่สามารถได้รับสิทธิ์เข้าถึง:

- **`/join` (ไม่มี code)** — ส่งการแจ้งเตือนไปยัง admin ทุกคนบน platform เดียวกัน
  การแจ้งเตือนรวมคำสั่ง `/role approve <platform:id> <role>` สำเร็จรูปที่ admin สามารถตอบกลับได้เลยเพื่อให้สิทธิ์เข้าถึง
- **`/join <code>`** — ใช้ invite code เพื่อกำหนด role ทันที โดยไม่ต้องมี admin online

### Invite Code

Admin สร้าง invite code ด้วย `/invite <role> [max_uses]` โดยค่าเริ่มต้น code ใช้ได้ครั้งเดียวและหมดอายุใน 24 ชั่วโมง
สามารถแชร์ผ่านช่องทางอื่น (เช่น กลุ่ม LINE, email, หรือช่องทางใดก็ได้) และใช้งานได้โดยไม่คำนึงว่าผู้รับจะ redeem ผ่าน platform ไหน

### คำสั่ง Runtime

| คำสั่ง | ผู้ใช้ | คำอธิบาย |
|---|---|---|
| `/whoami` | ทุกคน | แสดง platform ID และ role ปัจจุบันของคุณ |
| `/join` | ผู้ใช้ที่ยังไม่ได้ลงทะเบียน | ขอสิทธิ์เข้าถึง — แจ้งเตือน admin พร้อมคำสั่งอนุมัติสำเร็จรูป |
| `/join <code>` | ผู้ใช้ที่ยังไม่ได้ลงทะเบียน | ใช้ invite code เพื่อกำหนด role ทันที |
| `/role list` | admin | แสดงรายชื่อ user ทั้งหมดที่ถูกกำหนด role บน platform ปัจจุบัน |
| `/role add <platform:id> <role>` | admin | กำหนด role โดยตรง |
| `/role approve <platform:id> <role>` | admin | กำหนด role และแจ้งเตือน user |
| `/role remove <platform:id>` | admin | ลบ role ของ user |
| `/role deny <platform:id>` | admin | ยกเลิกสิทธิ์เข้าถึง (เหมือน remove) |
| `/invite <role> [max_uses]` | admin | สร้าง invite code ที่แชร์ได้ (ใช้ได้ 1 ครั้ง, หมดอายุ 24 ชม. เป็นค่าเริ่มต้น) |

---

## หมายเหตุด้านความปลอดภัย

### Terminal tool

`terminal_sandbox: none` (ค่าเริ่มต้น) รัน shell command **โดยตรงบน host OS** — command ที่ agent เลือกรันจะมีสิทธิ์เท่ากับ server process

- **สำหรับการพัฒนา / CLI ในเครื่อง:** ค่าเริ่มต้นใช้งานได้
- **สำหรับ production / หลายผู้ใช้:** ตั้งค่า `terminal_sandbox: docker` เพื่อ isolate การรัน command หรือปิด tool นี้ทั้งหมด:

```yaml
security:
  terminal_sandbox: docker   # แนะนำสำหรับ production

# หรือปิด tool ทั้งหมด:
disabled_tools: [terminal]
```

`approval_mode: smart` ตรวจสอบและ log การเรียก tool ที่มีความเสี่ยง แต่**ไม่บล็อก**การทำงาน หากต้องการบล็อก:

```yaml
security:
  approval_mode: deny        # บล็อก tool call ทุกอย่างที่ไม่ได้รับอนุญาต
```

### delegate_task recursion

`delegate_task` spawn sub-agent ถ้าไม่มีการจำกัดความลึก อาจเกิดการ delegate ซ้อนกันไม่สิ้นสุดได้ ค่าเริ่มต้น `max_delegation_depth: 1` หมายความว่า sub-agent สามารถ spawn ได้อีก 1 ระดับ ตั้งค่าเป็น `0` เพื่อป้องกันทั้งหมด:

```yaml
max_delegation_depth: 0   # sub-agent ไม่สามารถ delegate ต่อได้เลย
```

---

## ร่วมพัฒนา

Welcome, garudian! Garudust สร้างขึ้นโดยคนที่เชื่อว่า AI agent ควรเร็ว ส่วนตัว และอยู่ภายใต้การควบคุมของผู้ใช้ ทุกการมีส่วนร่วม — แก้ typo, เพิ่ม tool ใหม่, หรือ feature เต็ม — ทำให้มันดีขึ้นสำหรับทุกคน

### วิธีมีส่วนร่วม

| ด้าน | สิ่งที่ต้องทำ | ความยาก |
|------|--------------|---------|
| รายงานบัก | เปิด issue พร้อมขั้นตอนที่ทำให้เกิดปัญหา | ต่ำมาก |
| เอกสาร | แก้ typo, ปรับตัวอย่าง, แปลภาษา | ต่ำ |
| Hub tools | เพิ่ม script tool ใน [garudust-hub](https://github.com/garudust-org/garudust-hub) | ต่ำ |
| Skills | เขียน skill ที่ใช้ซ้ำได้และแชร์บน [agentskills.io](https://agentskills.io) | ต่ำ |
| Platform adapters | เพิ่มรองรับแพลตฟอร์มแชทใหม่ใน `garudust-platforms` | ปานกลาง |
| Transport providers | เพิ่ม LLM provider ใหม่ใน `garudust-transport` | ปานกลาง |
| Core features | Agent loop, memory, compression, tools | สูง |

### เริ่มต้น

```bash
git clone https://github.com/garudust-org/garudust-agent
cd garudust-agent
cargo build                   # build ทุก crate
cargo test --workspace        # รัน test ทั้งหมด
cargo clippy --workspace      # ตรวจ lint
```

**เพิ่ม built-in tool** — implement `Tool` ใน `crates/garudust-tools/src/toolsets/` แล้ว register ใน `ToolRegistry::new()` ปกติใช้ไฟล์เดียวไม่ถึง 100 บรรทัด

**เพิ่ม hub tool** — วาง `tool.yaml` + script ลง [garudust-hub](https://github.com/garudust-org/garudust-hub) ใต้ `tools/<name>/` ไม่ต้องใช้ Rust

**เพิ่ม LLM provider** — implement `ProviderTransport` (`chat` + `chat_stream`) ใน `crates/garudust-transport/src/` แล้วเชื่อมใน `registry.rs`

**เพิ่ม platform adapter** — implement `PlatformAdapter` (`send_message` + `start_listening`) ใน `crates/garudust-platforms/src/`

ดู [CONTRIBUTING.md](../../../CONTRIBUTING.md) สำหรับคู่มือละเอียดในแต่ละด้าน

### ชุมชน

- [Discord](https://discord.com/channels/1501414298449088745/1501414298893942877) — คุย ถามตอบ และแชร์ไอเดีย
- [Issues](https://github.com/garudust-org/garudust-agent/issues) — รายงานบักและ feature requests
- [Discussions](https://github.com/garudust-org/garudust-agent/discussions) — ไอเดียและข้อเสนอที่ต้องการถกเถียง
- [dev.to/garudust](https://dev.to/garudust) — บทความและบทเรียน

---

## License

MIT — ดู [LICENSE](../../../LICENSE)

---

## ผู้ร่วมพัฒนา

[![](https://contrib.rocks/image?repo=garudust-org/garudust-agent)](https://github.com/garudust-org/garudust-agent/graphs/contributors)

---

<div align="center">
  <img src="https://visitor-badge.laobi.icu/badge?page_id=garudust-org.garudust-agent&style=flat" alt="visitors"/>
</div>
