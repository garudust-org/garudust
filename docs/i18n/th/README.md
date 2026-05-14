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

AI agent runtime แบบ self-improving เขียนด้วย Rust — ส่งมอบเป็น binary เดียวขนาด ~10 MB ไม่มี runtime dependency ไบนารีเดียวจัดการได้ทุกอย่าง: แชทในเทอร์มินัล ตอบบน multi-platform (Telegram, Discord, Slack, LINE, WhatsApp) หรือเปิด REST + WebSocket API ขยายความสามารถได้ทันทีผ่าน Tool Hub หรือวาง YAML file เพื่อเพิ่ม tool เอง เชื่อมต่อ MCP server ใดก็ได้ หรือให้ agent เขียนและปรับปรุง skill ที่นำกลับมาใช้ซ้ำได้เอง ไม่มี telemetry ไม่ผูกติดกับ vendor — ข้อมูลของคุณส่งไปแค่ LLM provider ที่คุณเลือกเท่านั้น

### ตัวอย่างการใช้งาน

<div align="center">
  <img src="../../../assets/demo.svg" alt="Garudust demo"/>
</div>

---

## ทำไมต้อง Garudust?

- **ไบนารีขนาด ~10 MB, cold start < 20 ms** — ไฟล์เดียว ไม่ต้องพึ่ง runtime อื่นสำหรับใช้งานบนเครื่องท้องถิ่น
- **พัฒนาตัวเองได้** — เรียนรู้ความชอบของคุณ บันทึก workflow ที่ใช้ซ้ำได้เป็นสกิล และแก้ไขตัวเองโดยไม่ต้องบอกสองครั้ง
- **รองรับ agentskills.io standard** — ติดตั้ง skill จาก [agentskills.io](https://agentskills.io) hub หรือ GitHub repo ใดก็ได้ด้วยคำสั่งเดียว รองรับ `allowed-tools`, version pinning และ scripts ครบ
- **Tool Hub ติดตั้งง่าย** — เรียกดูและติดตั้ง script tool จากชุมชนได้ทันทีด้วย `garudust tool install <name>` ไม่ต้องจัดการ folder เอง
- **พูดภาษาของคุณ** — ตรวจจับภาษาไทย จีน ญี่ปุ่น อาหรับ เกาหลี และอื่น ๆ โดยอัตโนมัติ ไม่ต้องตั้งค่าเพิ่ม
- **เปลี่ยน LLM ด้วย env var เดียว** — รองรับ Anthropic, OpenRouter, AWS Bedrock, Ollama, vLLM, ThaiLLM หรือ endpoint ที่เข้ากันได้กับ OpenAI
- **ปลอดภัยตั้งแต่ต้น** — Docker sandbox, การบล็อคคำสั่งอันตรายแบบไม่มีข้อยกเว้น, ป้องกันการฝังคำสั่งผ่าน memory และการ redact secret อัตโนมัติจาก output ของ tool
- **รันได้ทุกที่** — TUI บนแล็ปท็อป, headless server, Docker, Telegram, Discord, Slack, Matrix, LINE, WhatsApp, HTTP
- **ประกอบต่อได้ง่าย** — แต่ละส่วนแยกเป็น crate อิสระ เพิ่ม tool, platform หรือ transport โดยไม่กระทบโค้ดส่วนอื่น

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

ต้องการ Rust 1.87+:

```bash
git clone https://github.com/garudust-org/garudust-agent
cd garudust-agent
cargo build --release
export PATH="$PATH:$(pwd)/target/release"
```

---

## เริ่มต้นใช้งาน

```bash
garudust setup   # wizard ตั้งค่าครั้งแรก — เลือก provider, บันทึก API key
garudust         # เริ่ม agent chat แบบ TUI
```

### 1 — TUI แบบโต้ตอบ

```bash
garudust
```

<div align="center">
  <img src="../../../assets/demo-tui.png" alt="Garudust TUI" width="800"/>
</div>

| ปุ่ม | การทำงาน |
|------|----------|
| `Enter` | ส่งข้อความ |
| `↑ ↓` | เลื่อนดูประวัติ |
| `/new` | ล้างประวัติ เริ่มเซสชันใหม่ |
| `/model <ชื่อ>` | เปลี่ยนโมเดลขณะใช้งาน |
| `/help` | แสดงคำสั่ง slash ทั้งหมด |
| `Ctrl+C` | ออกจากโปรแกรม |

### 2 — One-shot

```bash
garudust "สรุป git log จาก 7 วันที่ผ่านมาเป็น changelog"
```

output ออก stdout, exit code 0 เมื่อสำเร็จ ใช้กับ pipe ได้เลย

### 3 — Server / Docker with Platforms

```bash
# แบบพื้นฐาน
garudust-server --port 3000

# ด้วย Docker
echo "OPENROUTER_API_KEY=sk-or-..." > .env
docker compose up

# Production: sandbox + LINE bot + cron รายวัน
# 1. ใส่ secret ใน ~/.garudust/.env:  LINE_CHANNEL_TOKEN, LINE_CHANNEL_SECRET
# 2. เปิดใช้ adapter LINE ใน ~/.garudust/config.yaml:
#      platforms:
#        line: { enabled: true, port: 3002, webhook_path: /line }
GARUDUST_TERMINAL_SANDBOX=docker \
GARUDUST_API_KEY=my-secret-token \
GARUDUST_CRON_JOBS="0 9 * * *=โพสต์สรุปเช้าไปยัง LINE" \
GARUDUST_MEMORY_CRON="0 3 * * *" \
garudust-server --port 3000 --approval-mode smart

# เปิด LINE webhook ผ่าน ngrok (สำหรับพัฒนา)
ngrok http 3002
# Webhook URL: https://xxxx.ngrok-free.app/line  ← นำไปใส่ใน LINE Developers Console
```

<div align="center">
  <img src="../../../assets/demo-line.jpg" alt="LINE Demo" width="420"/>
</div>

---

## คำสั่ง CLI

```bash
garudust setup                              # wizard ตั้งค่าครั้งแรก
garudust doctor                             # ตรวจสอบ API key, การเชื่อมต่อ, DB
garudust config show                        # แสดง config ที่ใช้งานอยู่
garudust model                              # แสดงโมเดลปัจจุบันและเปลี่ยนแบบ interactive
garudust model anthropic/claude-opus-4-7   # เปลี่ยนโมเดลโดยตรง
garudust config set ANTHROPIC_API_KEY sk-ant-...          # API keys → .env
garudust config set provider vllm                         # model / provider / base_url → config.yaml
garudust config set base_url http://localhost:8000/v1
garudust config set server.port 3001                      # nested keys ใช้ dot notation
garudust config set cron.memory_consolidation "0 3 * * *"
garudust config set platforms.line.enabled true
garudust config set platforms.line.port 3002
```

---

## การตั้งค่า

การตั้งค่าที่ไม่ใช่ secret อยู่ใน `~/.garudust/config.yaml` ส่วน API key และ token อยู่ใน `~/.garudust/.env` — รัน `garudust setup` เพื่อตั้งค่าแบบโต้ตอบ ทั้งสองไฟล์โหลดอย่างปลอดภัยตอน startup และไม่ถูกส่งต่อไปยัง subprocess

### `~/.garudust/config.yaml`

```yaml
# โมเดลและ provider — ไม่ใช่ secret จึงอยู่ที่นี่ (ไม่ใช่ใน .env)
model: anthropic/claude-sonnet-4-6   # model identifier
provider: anthropic                  # anthropic | openrouter | vllm | ollama | thaillm | custom
base_url: https://your-vllm-host/v1  # จำเป็นสำหรับ vllm/ollama; เปิด proxy mode สำหรับ anthropic

security:
  terminal_sandbox: docker           # none (ค่าเริ่มต้น) | docker
  terminal_sandbox_image: ubuntu:24.04
  terminal_sandbox_opts:
    - "--network=none"               # ตัดการเชื่อมต่อเครือข่ายขาออกภายใน container
    - "--memory=512m"                # จำกัดหน่วยความจำ

nudge_interval: 5                    # เตือนให้บันทึก memory ทุก N iterations (0 = ปิด)

# ปิด toolset ทั้งหมด (ลด context สำหรับโมเดล context เล็ก)
# ที่มี: web, files, terminal, memory, skills, agent, browser, git, notes, json, mcp
disabled_toolsets: [browser, git, notes]

# ปิด tool เฉพาะรายตัว โดยไม่ลบทั้ง toolset
disabled_tools: [image_read, pdf_read, session_search]

# สำหรับโมเดล context เล็ก (เช่น 27K): ระบุขนาด context จริง
# agent จะจำกัด output token อัตโนมัติและ retry เมื่อ overflow
context_window: 27168

mcp_servers:
  - name: filesystem
    command: npx
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
  - name: postgres
    command: npx
    args: ["-y", "@modelcontextprotocol/server-postgres", "postgresql://localhost/mydb"]

# การตั้งค่า HTTP gateway — `--port` / `GARUDUST_PORT` override ได้
server:
  port: 3000

# Cron — งานประจำของ agent + งานดูแล memory. `--cron-jobs` /
# `--memory-cron` / `--memory-expiry-cron` (และ env var ที่สอดคล้องกัน) override ได้
cron:
  jobs:
    - schedule: "0 9 * * *"
      task: "เขียนสรุปเช้าและบันทึกไว้ที่ ~/briefing.md"
  memory_consolidation: "0 3 * * *"   # null/ไม่ใส่ = ปิด
  memory_expiry: "0 4 * * *"           # null/ไม่ใส่ = ปิด

# Webhook-based platform adapters — secret อยู่ใน .env, ตั้งค่าที่นี่
# garudust setup (โหมด Full) สร้าง block นี้ให้อัตโนมัติ
platforms:
  line:
    enabled: true
    port: 3002
    webhook_path: /line      # URL webhook ใน LINE Developers Console: https://your-host:3002/line
  whatsapp:
    enabled: false
    port: 3003
    webhook_path: /whatsapp
```

### การตั้งค่า Platform

#### Telegram bot

```bash
# ~/.garudust/.env
ANTHROPIC_API_KEY=sk-ant-...
TELEGRAM_TOKEN=123456789:AAFxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx

# เริ่มต้น
garudust-server --telegram-token $TELEGRAM_TOKEN --anthropic-key $ANTHROPIC_API_KEY
```

#### LINE Messaging API

Adapter ที่ใช้ webhook (LINE, WhatsApp, generic webhook) ตั้งค่าใน `~/.garudust/config.yaml` ใต้คีย์ `platforms.*` ส่วน secret ยังอยู่ใน `~/.garudust/.env`

```bash
# ~/.garudust/.env
OPENROUTER_API_KEY=sk-or-...
LINE_CHANNEL_TOKEN=<channel-access-token>
LINE_CHANNEL_SECRET=<32-char-hex-secret>
```

```yaml
# ~/.garudust/config.yaml
platforms:
  line:
    enabled: true
    port: 3002
    webhook_path: /line   # webhook รับที่ https://your-host:3002/line
```

```bash
garudust-server --port 3000
```

#### WhatsApp Business

```bash
# ~/.garudust/.env
ANTHROPIC_API_KEY=sk-ant-...
WHATSAPP_ACCESS_TOKEN=EAAxxxxxxx
WHATSAPP_PHONE_NUMBER_ID=123456789012345
WHATSAPP_VERIFY_TOKEN=my_verify_token
WHATSAPP_APP_SECRET=<32-char-hex-secret>   # ไม่บังคับ — ข้ามการตรวจ HMAC หากเว้นว่าง
```

```yaml
# ~/.garudust/config.yaml
platforms:
  whatsapp:
    enabled: true
    port: 3003
    webhook_path: /whatsapp
```

```bash
garudust-server --port 3000
```

#### หลายแพลตฟอร์มพร้อมกัน (Telegram + LINE + WhatsApp + HTTP webhook)

ทุก adapter รันในกระบวนการเดียวกัน — secret อยู่ใน `.env` ส่วน enable/port/path อยู่ใน `config.yaml` แพลตฟอร์มที่ `enabled: false` หรือไม่มี token จะถูกข้ามเงียบ ๆ

```bash
# ~/.garudust/.env
ANTHROPIC_API_KEY=sk-ant-...
TELEGRAM_TOKEN=123456789:AAFxxx
LINE_CHANNEL_TOKEN=<token>
LINE_CHANNEL_SECRET=<secret>
WHATSAPP_ACCESS_TOKEN=EAAxxx
WHATSAPP_PHONE_NUMBER_ID=123456789012345
WHATSAPP_VERIFY_TOKEN=my_verify_token
```

```yaml
# ~/.garudust/config.yaml — เปิดใช้ adapter ที่ใช้ webhook
platforms:
  webhook:
    enabled: true
    port: 3001
    webhook_path: /webhook
  line:
    enabled: true
    port: 3002
    webhook_path: /line
  whatsapp:
    enabled: true
    port: 3003
    webhook_path: /whatsapp
```

```bash
garudust-server --port 3000
```

> **เคล็ดลับ:** ใช้ `garudust setup` (โหมด 2 — Full) เพื่อตั้งค่าแบบโต้ตอบที่จะเขียน `~/.garudust/.env` และบล็อก `platforms.*` ใน `~/.garudust/config.yaml` ให้อัตโนมัติ

---

## ความปลอดภัย

### Terminal Sandbox

ตั้งค่า `terminal_sandbox: docker` ใน `config.yaml` เพื่อรันทุกคำสั่ง shell ภายใน container ที่แยกออกมา (`--cap-drop ALL`, `--pids-limit 256`, working directory mount ที่ `/workspace`) ต้องติดตั้ง Docker ไว้ก่อน

### การบล็อคคำสั่งอันตราย

บล็อคโดยไม่มีเงื่อนไข ไม่ว่าจะตั้ง approval mode แบบใด:

| รูปแบบ | ตัวอย่าง |
|--------|---------|
| ลบ root filesystem แบบ recursive | `rm -rf /`, `rm -rf /*` |
| Format filesystem | `mkfs`, `mkfs.ext4 /dev/sda1` |
| Fork bomb | `:(){ :|:& };:` |
| เขียนไปยัง raw block device | `dd of=/dev/sda`, `cat > /dev/nvme0n1` |
| ปิดเครื่อง / รีบูต | `shutdown`, `reboot`, `halt`, `systemctl poweroff` |
| เขียนไปยัง credential path | `~/.ssh/authorized_keys`, `~/.aws/credentials`, `~/.bashrc` |

### Approval Mode

| โหมด | พฤติกรรม |
|------|----------|
| `smart` *(ค่าเริ่มต้น)* | อนุมัติ tool ทั้งหมด; constitutional constraints เป็น gate หลัก; ทุก destructive call ถูก audit-log |
| `auto` | เหมือน `smart` — ใช้ใน automation pipeline ที่เชื่อถือได้ |
| `deny` | บล็อก destructive call ทั้งหมด — สำหรับ agent แบบอ่านอย่างเดียว |

ตั้งค่าด้วย `GARUDUST_APPROVAL_MODE` หรือ `--approval-mode`

Memory entry จากเซสชันก่อนหน้าถูกห่อด้วย tag `<untrusted_memory>` เพื่อป้องกันการโจมตีแบบ memory poisoning API key ถูก redact จาก tool output โดยอัตโนมัติ และ output ถูกตัดให้ไม่เกิน 50 KB เพื่อป้องกัน context flooding

---

## Memory และการพัฒนาตัวเอง

agent บันทึกความรู้ที่คงทนไว้ใน `~/.garudust/memory/` และโหลดมาตั้งแต่เริ่มเซสชัน — คุณไม่ต้องบอกซ้ำอีก:

```
คุณ: format JSON ด้วย 2-space indent เสมอ
agent: [บันทึกความจำ] เข้าใจแล้ว จะใช้ 2-space indent สำหรับ JSON ต่อจากนี้
```

| หมวดหมู่ | ตัวอย่าง |
|---------|---------|
| ความชอบ | รูปแบบ output, ภาษา, โทน, การเลือกเครื่องมือ |
| รายละเอียดโปรเจกต์ | paths, configs, conventions, quirks ที่รู้จัก |
| การแก้ไข | สิ่งที่คุณบอก agent ให้หยุดทำ — บันทึกทันที |

ตั้งค่าความถี่การเตือนบันทึก memory ด้วย `nudge_interval` ใน `config.yaml` (0 = ปิด)

---

## สกิล

ชุดคำแนะนำที่ใช้ซ้ำได้ เก็บไว้ใน `~/.garudust/skills/` และโหลดใหม่ทุกครั้งที่เรียกใช้

```
~/.garudust/skills/
  git-workflow/SKILL.md
  daily-standup/SKILL.md
  rust-code-review/SKILL.md
```

agent สแกนสกิลทั้งหมดก่อนทุกข้อความและโหลดสกิลที่เกี่ยวข้อง สร้างและแก้ไขไฟล์สกิลโดยอัตโนมัติเมื่อค้นพบหรือแก้ไข workflow

Garudust รองรับ [agentskills.io](https://agentskills.io) open standard — ใช้ skill ได้โดยตรงโดยไม่ต้องแปลงไฟล์ รวมถึง `allowed-tools` restrictions และการรัน `scripts/` ครบ

ติดตั้ง skill จาก agentskills.io hub หรือ GitHub repo ใดก็ได้ด้วยคำสั่งเดียว:

```bash
# จาก GitHub (owner/repo/path)
garudust skill install agentskills-org/hub/git-workflow

# จาก URL โดยตรง
garudust skill install https://example.com/skills/my-skill/SKILL.md

# จาก well-known endpoint
garudust skill install well-known:https://example.com --name my-skill

garudust skill list                      # ดู skill ที่ติดตั้งอยู่
garudust skill uninstall git-workflow    # ลบ skill
```

ตัวอย่าง `SKILL.md` ขั้นต่ำ:

```markdown
---
name: git-workflow
description: Git commit และ PR workflow แบบมีมาตรฐาน
version: 1.0.0
---

เขียน conventional commits เสมอ รันเทสก่อน push เสมอ
เปิด draft PR ก่อน แล้วค่อยทำเป็น ready เมื่อ CI ผ่าน
```

---

## Headless Server

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

<div align="center">
  <a href="https://core.telegram.org/bots"><img src="https://img.shields.io/badge/Telegram-2CA5E0?logo=telegram&logoColor=white&style=for-the-badge" alt="Telegram"/></a>
  <a href="https://discord.com/developers/applications"><img src="https://img.shields.io/badge/Discord-5865F2?logo=discord&logoColor=white&style=for-the-badge" alt="Discord"/></a>
  <a href="https://api.slack.com/apps"><img src="https://img.shields.io/badge/Slack-4A154B?logo=slack&logoColor=white&style=for-the-badge" alt="Slack"/></a>
  <a href="https://matrix.org"><img src="https://img.shields.io/badge/Matrix-000000?logo=matrix&logoColor=white&style=for-the-badge" alt="Matrix"/></a>
  <a href="https://developers.line.biz/console/"><img src="https://img.shields.io/badge/LINE-00C300?logo=line&logoColor=white&style=for-the-badge" alt="LINE"/></a>
  <a href="https://developers.facebook.com/docs/whatsapp/cloud-api"><img src="https://img.shields.io/badge/WhatsApp-25D366?logo=whatsapp&logoColor=white&style=for-the-badge" alt="WhatsApp"/></a>
  <img src="https://img.shields.io/badge/Webhook-6E7681?style=for-the-badge" alt="Webhook"/>
</div>

ตั้งค่า token ที่เกี่ยวข้องใน `~/.garudust/.env` แล้วสตาร์ท `garudust-server` — ทุก adapter รันในกระบวนการเดียวกันได้

| แพลตฟอร์ม | Token ที่ต้องการ |
|-----------|-----------------|
| Telegram | `TELEGRAM_TOKEN` |
| Discord | `DISCORD_TOKEN` |
| Slack | `SLACK_BOT_TOKEN`, `SLACK_APP_TOKEN` |
| Matrix | `MATRIX_HOMESERVER`, `MATRIX_USER`, `MATRIX_PASSWORD` |
| LINE | `LINE_CHANNEL_TOKEN`, `LINE_CHANNEL_SECRET` + `platforms.line.enabled: true` |
| WhatsApp | `WHATSAPP_ACCESS_TOKEN`, `WHATSAPP_PHONE_NUMBER_ID`, `WHATSAPP_VERIFY_TOKEN` + `platforms.whatsapp.enabled: true` |
| Webhook | เปิดโดยค่าเริ่มต้นที่ `POST /webhook` (พอร์ต 3001) — ปรับได้ผ่าน `platforms.webhook` |

**Telegram** — สร้างบอทผ่าน [@BotFather](https://t.me/botfather) แล้วคัดลอก token

**Discord** — สร้าง application ที่ [discord.com/developers](https://discord.com/developers/applications) เปิด **Message Content Intent** ในส่วน Bot แล้วคัดลอก token

**Slack** — สร้าง app ที่ [api.slack.com/apps](https://api.slack.com/apps) เปิด **Socket Mode** เพิ่ม scopes `chat:write channels:history im:history` แล้วติดตั้งใน workspace

**Matrix** — รองรับ homeserver ทุกประเภท (matrix.org, Synapse, Dendrite ฯลฯ)

**LINE** — สร้าง Messaging API channel ที่ [developers.line.biz](https://developers.line.biz/console/) คัดลอก **Channel access token** และ **Channel secret** ใส่ลงใน `~/.garudust/.env` จากนั้นเพิ่ม `platforms.line: { enabled: true, port: 3002, webhook_path: /line }` ใน `~/.garudust/config.yaml` และตั้ง Webhook URL ใน LINE console เป็น `https://your-host:3002/line`

**WhatsApp** — สร้าง Meta app ที่ [developers.facebook.com](https://developers.facebook.com/) เพิ่มผลิตภัณฑ์ **WhatsApp** คัดลอก **Access token** และ **Phone number ID** ใส่ลงใน `~/.garudust/.env` จากนั้นเพิ่ม `platforms.whatsapp: { enabled: true, port: 3003, webhook_path: /whatsapp }` ใน `~/.garudust/config.yaml` และตั้ง Webhook URL ใน Meta console เป็น `https://your-host:3003/whatsapp` หากต้องการตรวจสอบ HMAC signature ให้ตั้งค่า `WHATSAPP_APP_SECRET` ด้วย

---

## ผู้ให้บริการ LLM

| ผู้ให้บริการ | `config.yaml` | `.env` (เฉพาะ secret) | หมายเหตุ |
|-------------|--------------|----------------------|----------|
| Anthropic | `provider: anthropic` | `ANTHROPIC_API_KEY` | Native Messages API; เพิ่ม `base_url` เพื่อใช้ proxy |
| OpenRouter | `provider: openrouter` *(ค่าเริ่มต้น)* | `OPENROUTER_API_KEY` | โมเดลกว่า 200 รายการ |
| AWS Bedrock | `provider: bedrock` | `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` | Converse API, SigV4 |
| OpenAI Responses | `provider: codex` | `OPENAI_API_KEY` | endpoint `/v1/responses` |
| Ollama | `provider: ollama` + `base_url` | *(ไม่จำเป็น)* | บนเครื่อง ไม่ต้องใช้ key |
| vLLM | `provider: vllm` + `base_url` | `VLLM_API_KEY` | เซิร์ฟเวอร์ OpenAI-compatible บนเครื่อง |
| ThaiLLM | `provider: thaillm` | `THAILLM_API_KEY` | Thai LLM โดย NSTDA |
| OpenAI-compatible อื่น ๆ | `provider: custom` + `base_url` | API key ที่เกี่ยวข้อง | Generic OpenAI-compatible transport |

ตั้ง `model`, `provider` และ `base_url` ใน `config.yaml` — ใส่เฉพาะ API key ใน `~/.garudust/.env` เปลี่ยนโมเดลได้ตลอดเวลาด้วย `garudust model`

---

## เครื่องมือในตัว

| เครื่องมือ | คำอธิบาย |
|-----------|----------|
| `web_fetch` | ดึงข้อมูลจาก URL (หน้าสแตติก) |
| `web_search` | ค้นหาเว็บ — ใช้ Serper (Google) เมื่อตั้ง `SERPER_API_KEY`, ใช้ Brave Search เมื่อตั้ง `BRAVE_SEARCH_API_KEY`, ใช้ DuckDuckGo เป็น fallback |
| `browser` | ควบคุม Chrome/Chromium ผ่าน CDP — navigate, คลิก, พิมพ์, screenshot, รัน JS |
| `read_file` | อ่านไฟล์จากระบบไฟล์ |
| `write_file` | เขียนไฟล์ไปยังระบบไฟล์; credential path ที่ละเอียดอ่อนถูกบล็อคเสมอ |
| `list_directory` | แสดงรายการไฟล์และโฟลเดอร์; รองรับ glob pattern (`**/*.rs`) และจำกัดความลึก |
| `terminal` | รันคำสั่ง shell; ทำงานใน Docker sandbox เมื่อตั้งค่า `terminal_sandbox: docker` |
| `memory` | หน่วยความจำถาวรแบบ key-value (add / read / replace / remove) |
| `user_profile` | อ่านและอัปเดต user profile ที่ถาวร |
| `session_search` | ค้นหาแบบ full-text ข้ามการสนทนาในอดีต (SQLite FTS5) |
| `delegate_task` | สร้าง sub-agent แบบขนานสำหรับงานที่แบ่งย่อย |
| `skills_list` | แสดงรายการสกิลที่มีอยู่ |
| `skill_view` | โหลดคำแนะนำเต็มของสกิลตามชื่อ |
| `write_skill` | สร้างหรืออัปเดตสกิลใน `~/.garudust/skills/` |

**MCP tools** — เชื่อมต่อ [MCP](https://modelcontextprotocol.io) server ใด ๆ โดยเพิ่มในรายการ `mcp_servers` ใน `config.yaml` (ดูที่หัวข้อการตั้งค่า)

**Script tools** — เพิ่ม tool เองโดยไม่ต้องเขียน Rust วาง folder ที่มี `tool.yaml` และ script ลงใน `~/.garudust/tools/` แล้ว restart agent:

```
~/.garudust/tools/
└── get_weather/
    ├── tool.yaml   ← ชื่อ, คำอธิบาย, schema, คำสั่ง
    └── run.py      ← script (อ้างอิงเป็น ./run.py ใน command — ไม่บังคับ)
```

```yaml
# tool.yaml
name: get_weather
description: ดึงข้อมูลสภาพอากาศของเมือง
destructive: false
schema:
  type: object
  properties:
    city:
      type: string
  required: [city]
command: "curl -s wttr.in/{city}?format=3"
```

ค่า parameter จะถูก shell-quote อัตโนมัติ คำสั่งรันใน `current_dir` ของ tool folder และมี `$TOOL_DIR` ตั้งให้ ทำให้ `./run.py` และไฟล์ข้างเคียง resolve ได้ถูกต้อง

### Tool Hub

ติดตั้ง script tool จากชุมชนผ่าน [garudust-hub](https://github.com/garudust-org/garudust-hub) ด้วยคำสั่งเดียว ไม่ต้องสร้าง folder เอง:

```bash
garudust tool list                  # ดู tool ที่มีอยู่และที่ติดตั้งแล้ว
garudust tool install weather       # ดาวน์โหลดไปที่ ~/.garudust/tools/weather/
garudust tool install hash_text
garudust tool uninstall weather     # ลบ tool และ folder
garudust tool update                # อัปเดต tool ทั้งหมดที่ติดตั้งจาก hub
```

`garudust tool list` แสดง runtime ที่ต้องใช้และคำอธิบายแต่ละ tool:

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

tool ที่ติดตั้งจะถูกบันทึกใน `~/.garudust/tools/registry.json` และโหลดอัตโนมัติทุกครั้งที่ agent เริ่มทำงาน พร้อมกับ tool ที่สร้างเองด้วยมือ

| คำสั่ง | คำอธิบาย |
|--------|----------|
| `tool list` | แสดง tool ที่ติดตั้งแล้วและ tool ที่มีใน hub แบบเปรียบเทียบ |
| `tool list --offline` | แสดงเฉพาะ tool ที่ติดตั้งในเครื่อง (ไม่เรียก network) |
| `tool install <ชื่อ>` | ดาวน์โหลดจาก hub ไปที่ `~/.garudust/tools/<ชื่อ>/` |
| `tool install <ชื่อ> --hub <owner/repo>` | ติดตั้งจาก hub repository อื่น |
| `tool uninstall <ชื่อ>` | ลบ tool folder และ registry entry |
| `tool update` | ดาวน์โหลด tool ทุกตัวที่มาจาก hub ใหม่เป็น version ล่าสุด |

หากต้องการเพิ่ม tool เข้า hub เปิด PR ได้ที่ [garudust-org/garudust-hub](https://github.com/garudust-org/garudust-hub)

---

## สถาปัตยกรรม

```
  garudust (CLI)              garudust-server
  ┌────────────────────┐    ┌─────────────────────────────────────────────┐
  │  TUI / one-shot    │    │  HTTP /chat · /stream · /ws                 │
  │  setup · config    ├──┐ │  Telegram · Discord · Slack · Matrix · LINE · WhatsApp │
  │  doctor · model    │  │ │  Webhook · Cron                             │
  │  tool · skill      │  │ └──────────────────────────┬──────────────────┘
  └──────────┬─────────┘  │                            │
     install │            └─────────────┬──────────────┘
             │                          ▼
             │                 ┌─────────────────┐
             │                 │      Agent       │
             │                 │   run_loop()     │
             │                 └────────┬─────────┘
             │              ┌───────────┴───────────┐
             │              ▼                       ▼
             │┌──────────────────────┐  ┌─────────────────────────────────┐
             ││      Transport       │  │        ToolRegistry              │
             ││  Anthropic           │  │  web_fetch · web_search          │
             ││  OpenRouter          │  │  http_request · browser          │
             ││  AWS Bedrock         │  │  read_file · write_file          │
             ││  Codex               │  │  list_directory · terminal       │
             ││  Ollama · vLLM       │  │  memory · user_profile           │
             ││  ThaiLLM             │  │  session_search · delegate_task  │
             │└──────────────────────┘  │  script tools · skills           │
             │                          │  MCP (external)                  │
             │                          └─────────────┬───────────────────┘
             │                                        │
             │                            ┌───────────┴───────────┐
             │                            ▼                       ▼
             │                  ┌──────────────────┐  ┌──────────────────────┐
             └─────────────────▶│ FileMemoryStore   │  │      SessionDb       │
                                │ tools/ · skills/  │  │   SQLite + FTS5      │
                                │ memory/           │  └──────────────────────┘
                                └────────┬──────────┘
                                         ▲
                               ┌─────────┴──────────┐
                               │   garudust-hub      │
                               │  community tools    │
                               │  & skills packages  │
                               └────────────────────┘
```

### โครงสร้าง Crate

| Crate / Binary | บทบาท |
|---|---|
| `garudust-core` | Trait และ type ที่ใช้ร่วมกัน — ไม่มี I/O |
| `garudust-transport` | LLM adapter: Anthropic, OpenAI-compat, Bedrock, Codex, Ollama, vLLM, ThaiLLM |
| `garudust-tools` | Tool registry + toolset ในตัว (web, files, terminal, browser, …) |
| `garudust-memory` | `FileMemoryStore` (markdown) + `SessionDb` (SQLite + FTS5) |
| `garudust-agent` | Agent run loop, context compressor, prompt builder |
| `garudust-platforms` | Telegram, Discord, Slack, Matrix, LINE, WhatsApp, Webhook |
| `garudust-cron` | Cron scheduler |
| `garudust-gateway` | axum HTTP gateway — `/chat`, `/chat/stream`, `/chat/ws`, `/metrics` |
| `bin/garudust` | CLI: TUI โต้ตอบ, one-shot, `setup`, `config`, `doctor`, `model` |
| `bin/garudust-server` | Headless: ทุกแพลตฟอร์ม + HTTP gateway + cron ในกระบวนการเดียว |

---

## การมีส่วนร่วม

Garudust ออกแบบมาให้ขยายได้ง่าย — การเพิ่มเครื่องมือ ทรานสปอร์ต หรือ platform adapter มักแตะโค้ดแค่ crate เดียวและใช้โค้ดไม่ถึง 100 บรรทัด

### Issues สำหรับผู้เริ่มต้น

- **เครื่องมือใหม่** — ห่อ CLI หรือ API ใด ๆ เป็น `Tool` impl ใน `garudust-tools`
- **แพลตฟอร์มใหม่** — implement `PlatformAdapter` (เช่น Signal, WeChat)
- **ปรับปรุง TUI** — multi-line input, syntax highlighting, รองรับเมาส์
- **เทส** — integration tests, property tests, snapshot tests

```bash
git clone https://github.com/garudust-org/garudust-agent
cd garudust-agent
cargo build
cargo test --workspace
cargo clippy --workspace --all-targets -- -W clippy::all -W clippy::pedantic
```

อ่าน [CONTRIBUTING.md](../../../CONTRIBUTING.md) สำหรับแนวทางโค้ด, commit convention และ CI checklist ครบถ้วน

มีคำถามหรือพบบัค? เข้าร่วม [ชุมชน Discord](https://discord.com/channels/1501414298449088745/1501414298893942877) หรือเปิด [GitHub issue](https://github.com/garudust-org/garudust-agent/issues)

---

## ใบอนุญาต

MIT — ดูที่ [LICENSE](../../../LICENSE)

---

## ผู้มีส่วนร่วม

[![](https://contrib.rocks/image?repo=garudust-org/garudust-agent)](https://github.com/garudust-org/garudust-agent/graphs/contributors)

---

## ประวัติ Star

[![Star History Chart](https://api.star-history.com/svg?repos=garudust-org/garudust-agent&type=Date)](https://star-history.com/#garudust-org/garudust-agent&Date)

---

<div align="center">
  <img src="https://visitor-badge.laobi.icu/badge?page_id=garudust-org.garudust-agent&style=flat" alt="visitors"/>
</div>
