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

**ระบบรันไทม์ AI agent ที่โฮสต์เองได้ เขียนด้วย Rust**

แชทจากเทอร์มินัล เชื่อมต่อกับ Telegram / Discord / Slack / Matrix หรือเรียกใช้งานผ่าน HTTP — ทั้งหมดจากไบนารีเดียว

---

## ทำไมต้อง Garudust?

เฟรมเวิร์ก AI agent ส่วนใหญ่เขียนด้วย Python ขนาดหนัก และสตาร์ทช้า Garudust แตกต่างออกไป:

- **ไบนารีขนาด ~10 MB, cold start < 20 ms** — ไม่ต้องใช้ Python runtime หรือ Docker สำหรับการใช้งานบนเครื่องท้องถิ่น
- **เปลี่ยนผู้ให้บริการ LLM ด้วย env var เดียว** — รองรับ Anthropic, OpenRouter, AWS Bedrock, Ollama, vLLM หรือ endpoint ที่เข้ากันได้กับ OpenAI
- **รันได้ทุกที่** — TUI บนแล็ปท็อป, เซิร์ฟเวอร์ headless, Docker, Telegram, Discord, Slack, Matrix, HTTP
- **ประกอบต่อได้ง่าย** — แต่ละส่วนแยกเป็น crate อิสระ เพิ่มเครื่องมือ แพลตฟอร์ม หรือทรานสปอร์ตได้โดยไม่กระทบโค้ดส่วนอื่น

---

## การติดตั้ง

### ไบนารีสำเร็จรูป (แนะนำ)

ดาวน์โหลดได้จาก [**GitHub Releases**](https://github.com/garudust-org/garudust-agent/releases/latest) — ไม่ต้องติดตั้ง Rust:

| แพลตฟอร์ม | ไฟล์ |
|-----------|------|
| macOS Apple Silicon | `garudust-*-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `garudust-*-x86_64-apple-darwin.tar.gz` |
| Linux x86_64 | `garudust-*-x86_64-unknown-linux-musl.tar.gz` |
| Linux ARM64 | `garudust-*-aarch64-unknown-linux-musl.tar.gz` |
| Windows | `garudust-*-x86_64-pc-windows-msvc.zip` |

```bash
tar -xzf garudust-*.tar.gz
sudo mv garudust garudust-server /usr/local/bin/
```

### Build จาก source

ต้องการ Rust 1.75+:

```bash
git clone https://github.com/garudust-org/garudust-agent
cd garudust
cargo build --release
export PATH="$PATH:$(pwd)/target/release"
```

---

## เริ่มต้นใช้งาน

### 1. ตั้งค่าและแชท

```bash
garudust setup   # เลือก provider (OpenRouter / Anthropic / vLLM / Ollama / custom) + บันทึก key
garudust         # เปิด TUI แบบโต้ตอบ
```

### 2. Docker (server mode)

```bash
# Cloud provider
echo "OPENROUTER_API_KEY=sk-or-..." > .env
docker compose up

# โมเดลบนเครื่องผ่าน Ollama
echo "OLLAMA_BASE_URL=http://host.docker.internal:11434" > .env
echo "GARUDUST_MODEL=llama3.2" >> .env
docker compose up
```

```bash
curl -X POST http://localhost:3000/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "2+2 เท่ากับเท่าไร?"}'
```

---

## การใช้งาน CLI

### TUI แบบโต้ตอบ

```bash
garudust
```

| ปุ่ม | การทำงาน |
|------|----------|
| `Enter` | ส่งข้อความ |
| `↑ ↓` | เลื่อนดูประวัติ |
| `/new` | ล้างประวัติ เริ่มเซสชันใหม่ |
| `/model <ชื่อ>` | เปลี่ยนโมเดลขณะใช้งาน |
| `/help` | แสดงคำสั่ง slash ทั้งหมด |
| `Ctrl+C` | ออกจากโปรแกรม |

### คำสั่งแบบ one-shot

```bash
garudust "สรุป git log จาก 7 วันที่ผ่านมาเป็น changelog"
garudust --model anthropic/claude-opus-4-7 "ตรวจสอบ PR นี้ด้านความปลอดภัย"
```

### คำสั่ง config

```bash
garudust setup                              # wizard ตั้งค่า (Quick หรือ Full mode)
garudust doctor                             # ตรวจสอบ API key, การเชื่อมต่อ, DB
garudust config show                        # แสดง config ที่ใช้งานอยู่
garudust model                              # แสดง model ปัจจุบัน และเปลี่ยนแบบ interactive
garudust model anthropic/claude-opus-4-7   # เปลี่ยน model โดยตรง
garudust config set OPENROUTER_API_KEY sk-or-...
garudust config set ANTHROPIC_API_KEY sk-ant-...
garudust config set VLLM_BASE_URL http://localhost:8000/v1
```

---

## เซิร์ฟเวอร์แบบ Headless

`garudust-server` รัน HTTP gateway, platform adapter ทั้งหมด และ cron job ในกระบวนการเดียว

```bash
garudust-server --anthropic-key sk-ant-... --port 3000
```

### HTTP API

```bash
# แบบ blocking
curl -X POST http://localhost:3000/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "เขียน haiku เกี่ยวกับ Rust"}'

# Streaming (Server-Sent Events)
curl -X POST http://localhost:3000/chat/stream \
  -H "Content-Type: application/json" \
  -d '{"message": "อธิบาย async/await ใน 3 ประโยค"}'

# WebSocket: ws://localhost:3000/chat/ws
# ส่ง: {"message": "งานของคุณ"}  รับ: text chunks … จากนั้น {"done":true}

# Health & metrics
curl http://localhost:3000/health
curl http://localhost:3000/metrics   # รองรับ Prometheus
```

---

## Platform Adapter

ตั้งค่า env var ที่เกี่ยวข้องและสตาร์ท `garudust-server` — ทุก adapter รันในกระบวนการเดียวกันได้

| แพลตฟอร์ม | Env var ที่ต้องการ |
|-----------|-------------------|
| Telegram | `TELEGRAM_TOKEN` |
| Discord | `DISCORD_TOKEN` |
| Slack | `SLACK_BOT_TOKEN`, `SLACK_APP_TOKEN` |
| Matrix | `MATRIX_HOMESERVER`, `MATRIX_USER`, `MATRIX_PASSWORD` |
| Webhook | เปิดอยู่ที่ `POST /webhook` เสมอ — ไม่ต้องใช้ token |

**Telegram** — สร้างบอทผ่าน [@BotFather](https://t.me/botfather) แล้วคัดลอก token

**Discord** — สร้าง application ที่ [discord.com/developers](https://discord.com/developers/applications) เปิด **Message Content Intent** ในส่วน Bot แล้วคัดลอก token

**Slack** — สร้าง app ที่ [api.slack.com/apps](https://api.slack.com/apps) เปิด **Socket Mode** เพิ่ม scopes `chat:write channels:history im:history` แล้วติดตั้งใน workspace

**Matrix** — รองรับ homeserver ทุกประเภท (matrix.org, Synapse, Dendrite ฯลฯ)

```bash
TELEGRAM_TOKEN=123:ABC \
SLACK_BOT_TOKEN=xoxb-... \
SLACK_APP_TOKEN=xapp-... \
garudust-server --anthropic-key sk-ant-...
```

---

## ผู้ให้บริการ LLM

| ผู้ให้บริการ | วิธีเลือก | หมายเหตุ |
|-------------|-----------|----------|
| Anthropic | ตั้ง `ANTHROPIC_API_KEY` | Direct Messages API |
| OpenRouter | ตั้ง `OPENROUTER_API_KEY` *(ค่าเริ่มต้น)* | โมเดลกว่า 200 รายการ |
| AWS Bedrock | ตั้ง `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` | Converse API, SigV4 |
| OpenAI Responses | `garudust config set provider codex` | endpoint `/v1/responses` |
| Ollama | ตั้ง `OLLAMA_BASE_URL` | บนเครื่อง ไม่ต้องใช้ key |
| vLLM | ตั้ง `VLLM_BASE_URL` | เซิร์ฟเวอร์ OpenAI-compatible บนเครื่อง |
| OpenAI-compatible อื่น ๆ | ตั้ง `GARUDUST_BASE_URL` | Generic transport |

```bash
# Ollama (บนเครื่อง ไม่ต้องใช้ key)
OLLAMA_BASE_URL=http://localhost:11434
GARUDUST_MODEL=llama3.2

# vLLM
VLLM_BASE_URL=http://localhost:8000/v1
VLLM_API_KEY=token-abc123
GARUDUST_MODEL=meta-llama/Llama-3.1-8B-Instruct

# AWS Bedrock
garudust config set provider bedrock
garudust config set model anthropic.claude-3-5-sonnet-20241022-v2:0
```

---

## เครื่องมือในตัว

| เครื่องมือ | คำอธิบาย |
|-----------|----------|
| `web_fetch` | ดึงข้อมูลจาก URL (หน้าสแตติก) |
| `web_search` | ค้นหาผ่าน Brave Search API (`BRAVE_SEARCH_API_KEY`) |
| `browser` | ควบคุม Chrome/Chromium ผ่าน CDP — navigate, คลิก, พิมพ์, screenshot, รัน JS |
| `read_file` | อ่านไฟล์จากระบบไฟล์ |
| `write_file` | เขียนไฟล์ไปยังระบบไฟล์ |
| `terminal` | รันคำสั่ง shell |
| `memory` | หน่วยความจำถาวรแบบ key-value (add / read / replace / remove) |
| `session_search` | ค้นหาแบบ full-text ข้ามการสนทนาในอดีต (SQLite FTS5) |
| `delegate_task` | สร้าง sub-agent แบบขนานสำหรับงานที่แบ่งย่อย |
| `skills_list` | แสดงรายการสกิลที่มีอยู่ |
| `skill_view` | โหลดคำแนะนำของสกิลตามชื่อ |

### MCP Tools

เชื่อมต่อ [MCP](https://modelcontextprotocol.io) server ใด ๆ ใน `~/.garudust/config.yaml`:

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

## สกิล

สกิลคือชุดคำแนะนำที่นำมาใช้ซ้ำได้ เก็บไว้ใน `~/.garudust/skills/` อ่านจากดิสก์ทุกครั้งที่เรียกใช้งาน แก้ไขไฟล์สกิลแล้วการเรียกครั้งถัดไปจะรับการเปลี่ยนแปลงทันที

```
~/.garudust/skills/
  git-workflow/SKILL.md
  daily-standup/SKILL.md
```

ตัวอย่าง `SKILL.md` ขั้นต่ำ:

```markdown
---
name: git-workflow
description: Git commit และ PR workflow แบบมีมาตรฐาน
version: 1.0.0
---

เขียน conventional commits เสมอ รันเทสก่อน push เสมอ...
```

---

## Environment Variable ทั้งหมด

| ตัวแปร | ค่าเริ่มต้น | คำอธิบาย |
|--------|------------|----------|
| `ANTHROPIC_API_KEY` | — | Anthropic key (เลือก Anthropic transport อัตโนมัติ) |
| `OPENROUTER_API_KEY` | — | OpenRouter key (provider เริ่มต้น) |
| `OLLAMA_BASE_URL` | — | Ollama base URL — ไม่ต้องใช้ key |
| `VLLM_BASE_URL` | — | vLLM base URL |
| `VLLM_API_KEY` | — | vLLM API key (ไม่บังคับ) |
| `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` | — | Bedrock credentials |
| `BRAVE_SEARCH_API_KEY` | — | เปิดใช้เครื่องมือ `web_search` |
| `GARUDUST_MODEL` | `anthropic/claude-sonnet-4-6` | identifier ของโมเดล |
| `GARUDUST_PORT` | `3000` | พอร์ต HTTP gateway |
| `GARUDUST_WEBHOOK_PORT` | `3001` | พอร์ต Webhook adapter (`0` = ปิดใช้งาน) |
| `GARUDUST_BASE_URL` | — | Override LLM base URL |
| `GARUDUST_API_KEY` | — | Bearer token สำหรับ `/chat*` (แนะนำใน production) |
| `GARUDUST_APPROVAL_MODE` | `smart` | `auto` \| `smart` \| `deny` |
| `GARUDUST_RATE_LIMIT` | — | Rate limit ต่อ IP (requests/นาที) |
| `TELEGRAM_TOKEN` | — | Telegram bot token |
| `DISCORD_TOKEN` | — | Discord bot token |
| `SLACK_BOT_TOKEN` | — | Slack bot token (`xoxb-…`) |
| `SLACK_APP_TOKEN` | — | Slack app token (`xapp-…`) |
| `MATRIX_HOMESERVER` | — | Matrix homeserver URL |
| `MATRIX_USER` | — | Matrix username (`@bot:matrix.org`) |
| `MATRIX_PASSWORD` | — | Matrix password |
| `GARUDUST_CRON_JOBS` | — | คู่ `"cron_expr=task"` คั่นด้วยจุลภาค |
| `RUST_LOG` | `info` | ระดับ log (`debug` สำหรับ verbose) |

### Cron jobs

```bash
GARUDUST_CRON_JOBS="0 9 * * *=เขียนสรุปเช้าและบันทึกไว้ที่ ~/briefing.md" \
garudust-server --anthropic-key sk-ant-...
```

---

## สถาปัตยกรรม

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

### โครงสร้าง Crate

```
crates/
  garudust-core        trait และ type ที่ใช้ร่วมกัน — ไม่มี I/O
  garudust-transport   LLM adapter: Anthropic, OpenAI-compat, Codex, Bedrock, Ollama, vLLM
  garudust-tools       Tool registry + toolset ในตัว (web, browser, file, …)
  garudust-memory      FileMemoryStore (markdown) + SessionDb (SQLite + FTS5)
  garudust-agent       Agent run loop, context compressor, prompt builder
  garudust-platforms   Telegram, Discord, Slack, Matrix, Webhook
  garudust-cron        Cron scheduler
  garudust-gateway     axum HTTP gateway — /chat, /chat/stream, /chat/ws, /metrics

bin/
  garudust             CLI: TUI โต้ตอบ, one-shot, setup, doctor, config, model
  garudust-server      Headless: ทุกแพลตฟอร์ม + HTTP + cron ในกระบวนการเดียว
```

---

## การมีส่วนร่วม

Garudust ออกแบบมาให้ขยายได้ง่าย — การเพิ่มเครื่องมือ ทรานสปอร์ต หรือ platform adapter มักแตะโค้ดแค่ crate เดียวและใช้โค้ดไม่ถึง 100 บรรทัด

### Issues สำหรับผู้เริ่มต้น

- **เครื่องมือใหม่** — ห่อ CLI หรือ API ใด ๆ เป็น `Tool` impl ใน `garudust-tools`
- **แพลตฟอร์มใหม่** — implement `PlatformAdapter` (เช่น Signal, LINE, WhatsApp)
- **ปรับปรุง TUI** — multi-line input, syntax highlighting, รองรับเมาส์
- **เทส** — integration tests, property tests, snapshot tests

```bash
git clone https://github.com/garudust-org/garudust-agent
cd garudust
cargo build
cargo test --workspace
cargo clippy --workspace --all-targets -- -W clippy::all -W clippy::pedantic
```

อ่าน [CONTRIBUTING.md](../../../CONTRIBUTING.md) สำหรับแนวทางโค้ด, commit convention และ CI checklist ครบถ้วน

---

## ใบอนุญาต

MIT — ดูที่ [LICENSE](../../../LICENSE)
