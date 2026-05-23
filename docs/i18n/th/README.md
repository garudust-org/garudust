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

AI agent runtime แบบ self-improving เขียนด้วย Rust — binary เดียวขนาด ~10 MB ไม่มี runtime dependency แชทในเทอร์มินัล ตอบบน 7 แพลตฟอร์ม หรือเปิด REST + WebSocket API เปลี่ยน LLM provider ด้วย env var เดียว ไม่มี telemetry ไม่ผูกติด vendor

<div align="center">
  <img src="../../../assets/demo.svg" alt="Garudust demo"/>
</div>

---

## ระบบปฏิบัติการที่รองรับ

| OS | สถาปัตยกรรม | ไฟล์ |
|----|------------|------|
| macOS | Apple Silicon (M1/M2/M3/M4) | `garudust-*-aarch64-apple-darwin.tar.gz` |
| macOS | Intel | `garudust-*-x86_64-apple-darwin.tar.gz` |
| Linux | x86_64 | `garudust-*-x86_64-unknown-linux-musl.tar.gz` |
| Linux | ARM64 (Raspberry Pi 4/5, Jetson) | `garudust-*-aarch64-unknown-linux-musl.tar.gz` |
| Windows | x86_64 | `garudust-*-x86_64-pc-windows-msvc.zip` |

ดาวน์โหลดจาก [GitHub Releases](https://github.com/garudust-org/garudust-agent/releases/latest) — ไม่ต้องติดตั้ง Rust

---

## เริ่มใช้งาน

**01 — ติดตั้ง**

ดาวน์โหลด binary สำเร็จรูปจาก [GitHub Releases](https://github.com/garudust-org/garudust-agent/releases/latest) (macOS, Linux, Windows, ARM64):

```bash
ARCH=$(uname -m)
[ "$ARCH" = "aarch64" ] && TARGET="aarch64-unknown-linux-musl" || TARGET="x86_64-unknown-linux-musl"
VER=$(curl -s https://api.github.com/repos/garudust-org/garudust-agent/releases/latest | grep tag_name | cut -d'"' -f4)
curl -LO "https://github.com/garudust-org/garudust-agent/releases/latest/download/garudust-${VER}-${TARGET}.tar.gz"
tar -xzf garudust-*.tar.gz && sudo mv garudust garudust-server /usr/local/bin/
```

หรือ build จาก source (Rust 1.87+): `git clone https://github.com/garudust-org/garudust-agent && cargo build --release`

---

**02 — ตั้งค่า**

```bash
garudust setup    # wizard ตั้งค่าครั้งแรก — เลือก provider เขียน config.yaml + .env
```

หรือใส่ key ตรงใน `~/.garudust/.env` (เช่น `ANTHROPIC_API_KEY=sk-ant-...`) ดู [LLM Provider](#llm-provider) สำหรับ key ทั้งหมดที่รองรับ

---

**03 — รัน**

```bash
garudust                           # interactive TUI
garudust "สรุป git log"            # one-shot task
garudust --hint fast "ตรวจสอบนี้"  # ใช้ model ที่ถูกกว่า
garudust-server --port 3000        # headless REST + WebSocket server
docker compose up -d
```

<div align="center">
  <img src="../../../assets/demo-tui.png" alt="Garudust TUI" width="700"/>
</div>

| ปุ่ม | การทำงาน |
|------|----------|
| `Enter` | ส่งข้อความ |
| `↑ ↓` | เลื่อนประวัติ |
| `/new` | เริ่ม session ใหม่ |
| `/model <name>` | เปลี่ยน model |
| `Ctrl+C` | ออกจากโปรแกรม |

---

## ทำไมต้อง Garudust?

- **~10 MB, cold start < 20 ms** — ไฟล์เดียว ไม่ต้องพึ่ง runtime อื่น
- **พัฒนาตัวเองได้** — จดจำความชอบ สร้าง skill จาก workflow และแก้ไขตัวเองโดยไม่ต้องบอกซ้ำ
- **รัน tool พร้อมกัน** — tool ที่ไม่ขัดแย้งกันรันคู่ขนานอัตโนมัติ
- **24 LLM provider** — Anthropic, OpenAI, Gemini, Groq, Ollama, Bedrock และอีกมาก — เปลี่ยนด้วยบรรทัดเดียว
- **7 แพลตฟอร์มในกระบวนการเดียว** — Telegram, Discord, Slack, Matrix, LINE, WhatsApp, Webhook
- **ปลอดภัยตั้งแต่ต้น** — Docker sandbox, RBAC, per-user rate limit, redact secret อัตโนมัติ

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

ทุก adapter รันในกระบวนการเดียวกับ `garudust-server` ตั้ง token ใน `~/.garudust/.env` ก็พร้อมใช้งานทันที

---

## LLM Provider

ตั้งค่า `providers.default.name` ใน `config.yaml` และ key ที่เกี่ยวข้องใน `~/.garudust/.env`:

| Provider | `name` | `.env` key |
|----------|--------|------------|
| Anthropic | `anthropic` | `ANTHROPIC_API_KEY` |
| OpenAI | `openai` | `OPENAI_API_KEY` |
| Google Gemini | `gemini` | `GEMINI_API_KEY` |
| Groq | `groq` | `GROQ_API_KEY` |
| Mistral | `mistral` | `MISTRAL_API_KEY` |
| DeepSeek | `deepseek` | `DEEPSEEK_API_KEY` |
| xAI (Grok) | `xai` | `XAI_API_KEY` |
| OpenRouter | `openrouter` | `OPENROUTER_API_KEY` |
| AWS Bedrock | `bedrock` | `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` |
| Ollama | `ollama` | *(ไม่ต้องการ — เพิ่ม `url:` สำหรับ endpoint กำหนดเอง)* |
| vLLM | `vllm` | `VLLM_API_KEY` |
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
| OpenAI-compat อื่น ๆ | *(ไม่ใส่ `name:` ตั้ง `url:` ใน profile แทน)* | API key ที่เกี่ยวข้อง |

Fallback keys: ตั้ง `LLM_FALLBACK_API_KEYS=key2,key3` ใน `.env` — สลับอัตโนมัติเมื่อ auth ล้มเหลว

---

## สถาปัตยกรรม

```
┌──────────────────────────────────────────────────────────────────────┐
│  bin/garudust (CLI)              bin/garudust-server (Daemon)        │
└────────────────────┬─────────────────────────┬───────────────────────┘
                     │                         │
                     │          ┌──────────────┴───────────────────────┐
                     │          │  garudust-gateway  (server-only)     │
                     │          │  POST /chat · POST /stream · GET /ws │
                     │          │  RBAC · /join · /invite · Metrics    │
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
│  โหลด memory → สร้าง prompt → เรียก LLM → รัน tool → วนซ้ำ         │
└──────┬──────────────┬─────────────────┬─────────────────────────────┘
       ▼              ▼                 ▼
  garudust-      garudust-        garudust-
  transport      tools            memory
  (24 LLM +     (built-in +      (memory.md +
  key rotation)  hub + MCP)       SQLite + RAG)

garudust-core — shared types · config · traits (ใช้โดยทุก crate ข้างต้น)
```

---

## การตั้งค่า

Secret เก็บใน `~/.garudust/.env` ส่วนการตั้งค่าอื่น ๆ อยู่ใน `~/.garudust/config.yaml`

### `~/.garudust/.env`

```bash
# LLM provider — ตั้ง 1 ตัว (ถ้าไม่มี config.yaml จะ detect อัตโนมัติจาก env)
ANTHROPIC_API_KEY=sk-ant-...
# OPENAI_API_KEY=sk-...
# GEMINI_API_KEY=AIza...
# GROQ_API_KEY=gsk_...

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

# ค้นหา (optional — fallback เป็น DuckDuckGo)
BRAVE_SEARCH_API_KEY=BSA...
SERPER_API_KEY=...

# Gateway auth
GARUDUST_API_KEY=my-gateway-secret
```

### `~/.garudust/config.yaml`

```yaml
providers:
  default:
    name: anthropic          # ดูตาราง LLM Provider ด้านบนสำหรับ 24 ตัวเลือก
    key: ${ANTHROPIC_API_KEY}
    model: claude-sonnet-4-6

security:
  approval_mode: smart       # auto | smart | deny
  terminal_sandbox: none     # none | docker  ← ใช้ docker ใน production
  rate_limit_rpm: ~          # จำกัดต่อ IP (~ = ไม่จำกัด)
  rate_limit_rpm_per_user: ~ # จำกัดต่อ (platform, user_id)

# เปลี่ยน model เฉพาะ task โดยไม่กระทบ default:
routing:
  fast: groq-fast/llama-3.1-8b-instant
  # ใช้: garudust --hint fast "คำถามด่วน"
```

ดู config ครบทุก option ได้ที่ [CONTRIBUTING.md](../../../CONTRIBUTING.md)

---

## Tools

built-in tools พร้อมใช้ทันที:

`web_fetch` · `web_search` · `http_request` · `browser` (CDP) · `read_file` · `write_file` · `list_directory` · `terminal` · `memory` · `session_search` · `delegate_task` · `skill_view` · `write_skill` · `doc_ingest` · `doc_search`

**Hub** — tool และ skill จากชุมชนใน [garudust-hub](https://github.com/garudust-org/garudust-hub):

```bash
garudust tool install hash_text    # script tool → ~/.garudust/tools/hash_text/
garudust tool install read_qr
garudust skill install weather     # Markdown instruction ไม่ต้องใช้ subprocess
garudust skill install fetch-title
```

**MCP** — เชื่อมต่อ [Model Context Protocol](https://modelcontextprotocol.io) server ใดก็ได้:

```yaml
mcp_servers:
  - name: filesystem
    command: npx
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
```

**Custom tools** — วาง `tool.yaml` + script ใน `~/.garudust/tools/<name>/` ใช้ภาษาใดก็ได้ ดูตัวอย่างได้ที่ [garudust-hub](https://github.com/garudust-org/garudust-hub)

---

## การควบคุมการเข้าถึง

Role-based access ผ่าน `roles:` ใน `config.yaml` คนแรกที่ DM จะได้รับ role `admin` อัตโนมัติหากยังไม่มี user ถูกกำหนด

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

คำสั่ง runtime: `/whoami` · `/join [code]` · `/invite <role> [max_uses]` · `/role list|add|approve|remove`

> **Production:** ตั้ง `terminal_sandbox: docker` เพื่อ sandbox shell execution และ `max_delegation_depth: 0` เพื่อป้องกัน sub-agent chain

---

## Memory & Skills

agent บันทึกทุกสิ่งที่เรียนรู้ไว้ใน `~/.garudust/memory/` และโหลดทุก session — ไม่ต้องบอกซ้ำ workflow ที่ใช้ซ้ำได้จะถูกเขียนเป็น skill ใน `~/.garudust/skills/` โดยอัตโนมัติหลัง `auto_skill_threshold` iterations

---

## ร่วมพัฒนา

Garudust เขียนด้วย Rust และออกแบบมาเพื่อขยายได้ง่าย เลือกด้านที่สนใจ:

| ด้าน | ที่ไหน | ความยาก |
|------|--------|---------|
| Hub tool หรือ skill | [garudust-hub](https://github.com/garudust-org/garudust-hub) — `tool.yaml` + script | ต่ำ — ไม่ต้องใช้ Rust |
| รายงานบัก / เอกสาร | [Issues](https://github.com/garudust-org/garudust-agent/issues) | ต่ำมาก |
| LLM provider ใหม่ | `crates/garudust-transport/src/` — impl `ProviderTransport` (2 method) | ปานกลาง |
| Platform adapter ใหม่ | `crates/garudust-platforms/src/` — impl `PlatformAdapter` (2 method) | ปานกลาง |
| Built-in tool | `crates/garudust-tools/src/toolsets/` — impl `Tool`, register ใน `ToolRegistry::new()` | ปานกลาง (~100 บรรทัด) |
| Core features | Agent loop, memory, compression, gateway | สูง |

```bash
git clone https://github.com/garudust-org/garudust-agent
cd garudust-agent
cargo build && cargo test --workspace && cargo clippy --workspace
```

คู่มือละเอียดในแต่ละด้าน: [CONTRIBUTING.md](../../../CONTRIBUTING.md)

**ชุมชน:** [Discord](https://discord.com/channels/1501414298449088745/1501414298893942877) · [Issues](https://github.com/garudust-org/garudust-agent/issues) · [Discussions](https://github.com/garudust-org/garudust-agent/discussions) · [dev.to/garudust](https://dev.to/garudust)

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
