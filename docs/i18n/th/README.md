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

## ทำไมต้อง Garudust?

- **~10 MB, cold start < 20 ms** — ไฟล์เดียว ไม่ต้องพึ่ง runtime อื่น
- **พัฒนาตัวเองได้** — จดจำความชอบของคุณ สร้าง skill จาก workflow และแก้ไขตัวเองโดยไม่ต้องบอกซ้ำ
- **รัน tool พร้อมกัน** — จัดกลุ่มตาม `parallelism_key` ทำงานคู่ขนานโดยอัตโนมัติ serializes เฉพาะที่จำเป็น
- **หมุนเวียน API key อัตโนมัติ** — ตั้ง `LLM_FALLBACK_API_KEYS` แล้ว agent จะสลับ key เมื่อเจอ auth error โดยไม่หยุดทำงาน
- **บีบอัด context อัจฉริยะ** — แบ่ง 3 zone: เก็บ task เดิมและ turn ล่าสุดไว้ สรุปเฉพาะตรงกลาง
- **Lifecycle hooks** — `AgentHooks` callback ทุก turn, compression, delegation และ session end
- **รองรับ agentskills.io** — ติดตั้ง skill จาก hub หรือ GitHub repo ใดก็ได้ด้วยคำสั่งเดียว
- **7 แพลตฟอร์มในกระบวนการเดียว** — Telegram, Discord, Slack, Matrix, LINE, WhatsApp, Webhook
- **เปลี่ยน LLM ด้วย env var เดียว** — รองรับ Anthropic, OpenAI, Gemini, Groq, Mistral, DeepSeek, xAI, OpenRouter, AWS Bedrock, Ollama, vLLM, ThaiLLM
- **Provider routing hints** — กำหนด hint name → provider/model ใน config แล้วส่ง `--hint fast` เพื่อเปลี่ยน model เฉพาะ task นั้นโดยไม่กระทบ default
- **กำหนด model ต่อ tool** — override model (และ fallback) ที่แต่ละ hub tool ใช้ผ่าน `tools.<name>.model` ใน `config.yaml`
- **ปลอดภัยตั้งแต่ต้น** — Docker sandbox, บล็อคคำสั่งอันตราย, ป้องกัน memory poisoning, redact secret อัตโนมัติ

---

## ติดตั้ง

ดาวน์โหลด binary สำเร็จรูปจาก [**GitHub Releases**](https://github.com/garudust-org/garudust-agent/releases/latest) — ไม่ต้องติดตั้ง Rust:

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

หรือ build จาก source (ต้องการ Rust 1.87+):

```bash
git clone https://github.com/garudust-org/garudust-agent
cd garudust-agent && cargo build --release
```

---

## เริ่มใช้งาน

```bash
garudust setup   # ตั้งค่าครั้งแรก — เลือก provider, บันทึก API key
```

### 1 — Interactive TUI

```bash
garudust
```

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

### 2 — One-shot

```bash
garudust "สรุป git log 7 วันล่าสุดเป็น changelog"
garudust --hint fast "ฟังก์ชันนี้ถูกต้องไหม?"   # ใช้ model ถูกกว่าสำหรับ task นี้
```

output ออก stdout รหัสออก 0 เมื่อสำเร็จ ใช้ pipe ได้เลย

### 3 — Server

`garudust-server` เปิด `POST /chat`, `POST /chat/stream` และ `ws://…/chat/ws` พร้อมรัน platform adapter ทุกตัวในกระบวนการเดียว ดูตัวอย่าง `.env` และ `config.yaml` ได้ที่หัวข้อ [การตั้งค่า](#การตั้งค่า)

**Binary**

```bash
garudust-server --port 3000
```

**Docker** (binary เดียวกัน ใส่ container)

```bash
# 1. สร้างไฟล์ .env (ดูตัวอย่างได้ที่หัวข้อการตั้งค่า)
cp .env.example .env   # หรือสร้างเอง
# 2. เริ่มต้น
docker compose up -d
# 3. ตรวจสอบ
curl http://localhost:3000/health
```

ข้อมูลถูกเก็บใน Docker volume `garudust-data` (`/root/.garudust` ใน container) หากต้องการใช้ `config.yaml` เอง ให้ bind-mount:

```yaml
# docker-compose.yml (เพิ่มใน volumes block)
- ./config.yaml:/root/.garudust/config.yaml:ro
```

**ทดสอบ API**

```bash
curl -X POST http://localhost:3000/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "เขียน haiku เกี่ยวกับ Rust"}'

# Streaming (Server-Sent Events)
curl -X POST http://localhost:3000/chat/stream \
  -H "Content-Type: application/json" \
  -d '{"message": "อธิบาย async/await ใน 3 ประโยค"}'
```

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
# ── LLM ─────────────────────────────────────────────────────────────────────
provider: openrouter        # anthropic | openai | gemini | groq | mistral
                            # deepseek | xai | openrouter | ollama | vllm | thaillm | bedrock
model: anthropic/claude-sonnet-4-6
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
# เพื่อเปลี่ยน provider/model เฉพาะ task นั้นโดยไม่กระทบ default
routing:
  fast:   groq/llama-3.1-8b-instant
  vision: openrouter/google/gemini-flash-1.5
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
  terminal_sandbox: none      # none | docker
  rate_limit_rpm: ~           # จำกัด request ต่อ IP ต่อนาที (~ = ไม่จำกัด)
  allowed_read_paths: []      # default: cwd + home
  allowed_write_paths: []     # default: cwd

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
  memory_consolidation: "0 3 * * *"   # housekeeping memory ทุกคืน
  memory_expiry: "0 4 * * 0"          # ลบ memory ที่หมดอายุรายสัปดาห์
  jobs:
    - schedule: "0 9 * * 1-5"
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

## ใหม่ใน v0.4.0

| ฟีเจอร์ | รายละเอียด |
|---|---|
| รัน tool พร้อมกัน | จัดกลุ่มด้วย `parallelism_key` — ทำงานคู่ขนาน serialize เฉพาะที่ขัดแย้ง |
| หมุนเวียน API key | `LLM_FALLBACK_API_KEYS=key2,key3` — สลับอัตโนมัติเมื่อ auth ล้มเหลว |
| บีบอัด context 3 zone | เก็บ task ต้นทาง + tail ล่าสุด สรุปเฉพาะตรงกลาง |
| `AgentHooks` trait | `on_turn_start`, `on_session_end`, `on_pre_compress`, `on_delegation` |
| Reasoning effort ขยาย | `Minimal` (512 tokens) → `Low` → `Medium` → `High` → `XHigh` (32k tokens) |
| งบ iteration ของ sub-agent | `sub_agent_max_iterations` แยกอิสระจาก agent หลัก |
| FTS5 trigram search | ค้นหาแบบ substring — `"pythag"` เจอ `"Pythagorean"` พร้อม migration อัตโนมัติ |
| WAL mode fallback | ลดระดับอัจฉริยะบน NFS/SMB แทนที่จะ crash |
| Provider routing hints | `routing:` ใน config กำหนด hint → provider/model; `--hint <name>` เปลี่ยน model เฉพาะ task นั้น |
| Per-tool model config | `tools.<name>.model` ใน config ส่ง `GARUDUST_MODEL`/`GARUDUST_FALLBACK_MODEL` ให้ subprocess ของ tool |

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

| Provider | `config.yaml` | `.env` |
|----------|--------------|--------|
| Anthropic | `provider: anthropic` | `ANTHROPIC_API_KEY` |
| OpenAI | `provider: openai` | `OPENAI_API_KEY` |
| Google Gemini | `provider: gemini` | `GEMINI_API_KEY` |
| Groq | `provider: groq` | `GROQ_API_KEY` |
| Mistral | `provider: mistral` | `MISTRAL_API_KEY` |
| DeepSeek | `provider: deepseek` | `DEEPSEEK_API_KEY` |
| xAI (Grok) | `provider: xai` | `XAI_API_KEY` |
| OpenRouter | `provider: openrouter` *(default)* | `OPENROUTER_API_KEY` |
| AWS Bedrock | `provider: bedrock` | `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` |
| Ollama | `provider: ollama` + `base_url` | *(ไม่ต้องการ)* |
| vLLM | `provider: vllm` + `base_url` | `VLLM_API_KEY` |
| ThaiLLM | `provider: thaillm` | `THAILLM_API_KEY` |
| OpenAI-compat อื่น ๆ | `provider: custom` + `base_url` | API key ที่เกี่ยวข้อง |

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
| `terminal` | รัน shell command (รองรับ Docker sandbox) |
| `memory` | memory แบบ key-value ที่คงอยู่ข้าม session |
| `session_search` | ค้นหาประวัติการสนทนา (FTS5 trigram) |
| `delegate_task` | spawn sub-agent แบบ parallel สำหรับงานย่อย |
| `skill_view` / `write_skill` | โหลดและเขียน skill ที่ใช้ซ้ำได้ |

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

กำหนด model สำหรับแต่ละ tool ใน `config.yaml` — ค่าจะถูกส่งเป็น env var `GARUDUST_MODEL` / `GARUDUST_FALLBACK_MODEL` ให้ script:

```yaml
tools:
  get_weather:
    model: groq/llama-3.1-8b-instant
    fallback_model: openrouter/meta-llama/llama-3.1-8b-instruct
```

**MCP** — เชื่อมต่อ [Model Context Protocol](https://modelcontextprotocol.io) server ใดก็ได้ใน `config.yaml`:

```yaml
mcp_servers:
  - name: filesystem
    command: npx
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
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

## ร่วมพัฒนา

การเพิ่ม tool, transport หรือ platform adapter โดยทั่วไปแก้ไขเพียง crate เดียว ใช้โค้ดไม่ถึง 100 บรรทัด ดู [CONTRIBUTING.md](../../../CONTRIBUTING.md) สำหรับคำแนะนำ

พบบัก หรือมีคำถาม? [เปิด issue](https://github.com/garudust-org/garudust-agent/issues) หรือเข้าร่วม [ชุมชน Discord](https://discord.com/channels/1501414298449088745/1501414298893942877)

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
