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

**ระบบรันไทม์ AI agent ที่โฮสต์เองได้ พัฒนาตัวเองได้ เขียนด้วย Rust**

แชทจากเทอร์มินัล เชื่อมต่อกับ Telegram / Discord / Slack / Matrix / LINE หรือเรียกใช้ผ่าน HTTP — ทั้งหมดจากไบนารีเดียว มันจำสิ่งที่คุณสอน พูดภาษาของคุณ และฉลาดขึ้นทุกเซสชัน

---

## ทำไมต้อง Garudust?

- **ไบนารีขนาด ~10 MB, cold start < 20 ms** — ไฟล์เดียว ไม่ต้องพึ่ง runtime อื่นสำหรับใช้งานบนเครื่องท้องถิ่น
- **พัฒนาตัวเองได้** — เรียนรู้ความชอบของคุณ บันทึก workflow ที่ใช้ซ้ำได้เป็นสกิล และแก้ไขตัวเองโดยไม่ต้องบอกสองครั้ง
- **พูดภาษาของคุณ** — ตรวจจับภาษาไทย จีน ญี่ปุ่น อาหรับ เกาหลี และอื่น ๆ โดยอัตโนมัติ ไม่ต้องตั้งค่าเพิ่ม
- **เปลี่ยน LLM ด้วย env var เดียว** — รองรับ Anthropic, OpenRouter, AWS Bedrock, Ollama, vLLM หรือ endpoint ที่เข้ากันได้กับ OpenAI
- **ปลอดภัยตั้งแต่ต้น** — Docker sandbox, การบล็อคคำสั่งอันตรายแบบไม่มีข้อยกเว้น, ป้องกันการฝังคำสั่งผ่าน memory และการ redact secret อัตโนมัติจาก output ของ tool
- **รันได้ทุกที่** — TUI บนแล็ปท็อป, headless server, Docker, Telegram, Discord, Slack, Matrix, LINE, HTTP
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

ต้องการ Rust 1.75+:

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
```

| | **1 — TUI** | **2 — One-shot** | **3 — Server / Docker** |
|---|---|---|---|
| **คำสั่ง** | `garudust` | `garudust "task"` | `garudust-server` |
| **ใช้เมื่อ** | แชทโต้ตอบส่วนตัว | Script / pipe / CI | Bot, API, cron job |
| **อินเทอร์เฟซ** | Terminal UI | stdout + exit code | HTTP + chat platform |

### 1 — TUI แบบโต้ตอบ

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

### 2 — One-shot

```bash
garudust "สรุป git log จาก 7 วันที่ผ่านมาเป็น changelog"
```

output ออก stdout, exit code 0 เมื่อสำเร็จ ใช้กับ pipe ได้เลย

### 3 — Server / Docker

```bash
# แบบพื้นฐาน
garudust-server --port 3000

# ด้วย Docker
echo "OPENROUTER_API_KEY=sk-or-..." > .env
docker compose up

# Production: sandbox + Telegram bot + cron รายวัน
GARUDUST_TERMINAL_SANDBOX=docker \
GARUDUST_API_KEY=my-secret-token \
TELEGRAM_TOKEN=123:ABC \
GARUDUST_CRON_JOBS="0 9 * * *=โพสต์สรุปเช้าไปยัง Telegram" \
GARUDUST_MEMORY_CRON="0 3 * * *" \
garudust-server --port 3000 --approval-mode smart
```

---

## คำสั่ง CLI

```bash
garudust setup                              # wizard ตั้งค่า
garudust doctor                             # ตรวจสอบ API key, การเชื่อมต่อ, DB
garudust config show                        # แสดง config ที่ใช้งานอยู่
garudust model                              # แสดงโมเดลปัจจุบันและเปลี่ยนแบบ interactive
garudust model anthropic/claude-opus-4-7   # เปลี่ยนโมเดลโดยตรง
garudust config set ANTHROPIC_API_KEY sk-ant-...
garudust config set VLLM_BASE_URL http://localhost:8000/v1
```

---

## การตั้งค่า

การตั้งค่าถาวรทั้งหมดอยู่ใน `~/.garudust/config.yaml` ส่วน secret และ token อยู่ใน `~/.garudust/.env` — รัน `garudust setup` เพื่อตั้งค่าแบบโต้ตอบ ทั้งสองไฟล์โหลดอย่างปลอดภัยตอน startup และไม่ถูกส่งต่อไปยัง subprocess

### `~/.garudust/config.yaml`

```yaml
model: anthropic/claude-sonnet-4-6   # model identifier
provider: anthropic                  # ตรวจจับอัตโนมัติจาก API key หากไม่ระบุ

security:
  terminal_sandbox: docker           # none (ค่าเริ่มต้น) | docker
  terminal_sandbox_image: ubuntu:24.04
  terminal_sandbox_opts:
    - "--network=none"               # ตัดการเชื่อมต่อเครือข่ายขาออกภายใน container
    - "--memory=512m"                # จำกัดหน่วยความจำ

nudge_interval: 5                    # เตือนให้บันทึก memory ทุก N iterations (0 = ปิด)

mcp_servers:
  - name: filesystem
    command: npx
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
  - name: postgres
    command: npx
    args: ["-y", "@modelcontextprotocol/server-postgres", "postgresql://localhost/mydb"]
```

## ความปลอดภัย

Garudust ออกแบบมาให้ปลอดภัยเมื่อ agent มีสิทธิ์เข้าถึง tool จริง — ระบบไฟล์, เทอร์มินัล และเว็บ

### Terminal Sandbox

เมื่อตั้งค่า `terminal_sandbox: docker` ทุกคำสั่ง shell จะรันภายใน container ที่แยกออกมาด้วย flag ที่เข้มงวด: `--cap-drop ALL`, `--security-opt no-new-privileges:true`, `--pids-limit 256` และ `/tmp` ชั่วคราว ไดเรกทอรีการทำงานปัจจุบันถูก mount ไปที่ `/workspace` ดังนั้นการดำเนินการกับไฟล์ยังทำงานได้

> **หมายเหตุ:** ต้องติดตั้ง Docker และเปิดใช้งานอยู่ หากไม่มี Docker จะแสดง error ชัดเจนทั้งตอน startup และตอนเรียกใช้ tool

### การบล็อคคำสั่งอันตราย

รูปแบบต่อไปนี้ถูกบล็อคโดยไม่มีเงื่อนไข ไม่ว่าจะตั้ง approval mode หรือ sandbox แบบใด:

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
| `smart` *(ค่าเริ่มต้น)* | อนุมัติ tool ทั้งหมด; constitutional constraints ใน system prompt เป็น gate หลัก; ทุก destructive call ถูก audit-log |
| `auto` | เหมือน `smart` — ใช้ใน automation pipeline ที่เชื่อถือได้ |
| `deny` | บล็อก destructive tool call ทุกอันโดยไม่มีเงื่อนไข — เหมาะสำหรับ agent แบบอ่านอย่างเดียว |

ตั้งค่าด้วย `GARUDUST_APPROVAL_MODE` หรือ `--approval-mode`

### การป้องกัน Memory

Memory entry ที่ดึงมาจากเซสชันก่อนหน้าจะถูกห่อด้วย tag `<untrusted_memory>` เพื่อให้โมเดลถือว่าเป็นข้อมูลจากผู้ใช้ ไม่ใช่คำสั่งที่เชื่อถือได้ ซึ่งป้องกันการโจมตีแบบ memory poisoning ที่ output ของ tool ที่เป็นอันตรายฝัง jailbreak ไว้ใน memory ถาวร นอกจากนี้ยังมีการ validate ตอนเขียนที่จะปฏิเสธ entry ที่มี XML control tag

### การ Redact Output

API key และ secret ที่โหลดตอน startup จะถูกลบออกจาก output ของ terminal command โดยอัตโนมัติก่อนส่งถึงโมเดล output ยังถูกตัดให้ไม่เกิน 50 KB (40% ส่วนหัว + 60% ส่วนท้าย) เพื่อป้องกัน context flooding

---

## หน่วยความจำและการพัฒนาตัวเอง

Garudust จำข้อมูลข้ามเซสชันและฉลาดขึ้นตามการใช้งาน

### หน่วยความจำทำงานอย่างไร

agent บันทึกความรู้ที่คงทนไว้ใน `~/.garudust/memory/` โดยอัตโนมัติ — ความชอบของผู้ใช้ สถาปัตยกรรมโปรเจกต์ และการแก้ไขที่คุณทำกับพฤติกรรมของมัน:

```
คุณ: format JSON ด้วย 2-space indent เสมอ
agent: [บันทึกความจำ] เข้าใจแล้ว จะใช้ 2-space indent สำหรับ JSON ต่อจากนี้
```

เซสชันถัดไป ความชอบนั้นโหลดมาแล้ว คุณไม่ต้องบอกซ้ำอีก ระบบ nudge จะทำงานทุก ๆ ไม่กี่ iteration ระหว่างงานที่ยาวเพื่อเตือนให้ agent บันทึกข้อเท็จจริงใหม่ก่อนจบเซสชัน ตั้งค่าด้วย `nudge_interval` ใน `config.yaml` (0 = ปิด)

### สิ่งที่ถูกบันทึก

| หมวดหมู่ | ตัวอย่าง |
|---------|---------|
| ความชอบ | รูปแบบ output, ภาษา, โทน, การเลือกเครื่องมือ |
| รายละเอียดโปรเจกต์ | paths, configs, conventions, quirks ที่รู้จัก |
| การแก้ไข | สิ่งที่คุณบอก agent ให้หยุดทำ — บันทึกทันที |

agent จะ **ไม่** บันทึก session log, ความคืบหน้างาน หรือรายละเอียดชั่วคราว — เฉพาะข้อเท็จจริงที่จะสำคัญในเซสชันอนาคต

---

## สกิล

สกิลคือชุดคำแนะนำที่ใช้ซ้ำได้ซึ่ง agent โหลดก่อนลงมือทำ เก็บไว้ใน `~/.garudust/skills/` และโหลดใหม่ทุกครั้งที่เรียกใช้ — แก้ไขไฟล์แล้วข้อความถัดไปรับการเปลี่ยนแปลงทันที

```
~/.garudust/skills/
  git-workflow/SKILL.md
  daily-standup/SKILL.md
  rust-code-review/SKILL.md
```

### การโหลดสกิลเชิงรุก

ก่อนประมวลผลทุกข้อความ agent จะสแกนสกิลที่มีทั้งหมด หากสกิลใดเกี่ยวข้อง — แม้แต่บางส่วน — มันจะเรียก `skill_view` เพื่อโหลดคำแนะนำเต็มก่อนดำเนินการ สิ่งนี้ทำให้ workflow ที่กำหนดถูกปฏิบัติตามเสมอ ไม่ว่าจะเขียนคำสั่งเป็นภาษาใด

### การสร้างสกิล

agent สร้างสกิลโดยอัตโนมัติเมื่อค้นพบ workflow หลายขั้นตอน:

```
คุณ: สร้างสกิลสำหรับ review Rust PR
agent: [เรียก write_skill] บันทึกสกิล 'rust-code-review' ไปยัง ~/.garudust/skills/rust-code-review/SKILL.md แล้ว
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

### การอัปเดตสกิล

หาก agent พบว่าขั้นตอนในสกิลล้าสมัยหรือผิดพลาดระหว่างงาน มันจะแก้ไขไฟล์ทันที ไม่ต้องรอให้ถาม สกิลจะถูกดูแลให้ถูกต้องโดยอัตโนมัติ

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

ตั้งค่า token ที่เกี่ยวข้องใน `~/.garudust/.env` แล้วสตาร์ท `garudust-server` — ทุก adapter รันในกระบวนการเดียวกันได้

| แพลตฟอร์ม | Token ที่ต้องการ |
|-----------|-----------------|
| Telegram | `TELEGRAM_TOKEN` |
| Discord | `DISCORD_TOKEN` |
| Slack | `SLACK_BOT_TOKEN`, `SLACK_APP_TOKEN` |
| Matrix | `MATRIX_HOMESERVER`, `MATRIX_USER`, `MATRIX_PASSWORD` |
| LINE | `LINE_CHANNEL_TOKEN`, `LINE_CHANNEL_SECRET` |
| Webhook | เปิดอยู่ที่ `POST /webhook` เสมอ — ไม่ต้องใช้ token |

**Telegram** — สร้างบอทผ่าน [@BotFather](https://t.me/botfather) แล้วคัดลอก token

**Discord** — สร้าง application ที่ [discord.com/developers](https://discord.com/developers/applications) เปิด **Message Content Intent** ในส่วน Bot แล้วคัดลอก token

**Slack** — สร้าง app ที่ [api.slack.com/apps](https://api.slack.com/apps) เปิด **Socket Mode** เพิ่ม scopes `chat:write channels:history im:history` แล้วติดตั้งใน workspace

**Matrix** — รองรับ homeserver ทุกประเภท (matrix.org, Synapse, Dendrite ฯลฯ)

**LINE** — สร้าง Messaging API channel ที่ [developers.line.biz](https://developers.line.biz/console/) คัดลอก **Channel access token** และ **Channel secret** จากนั้นตั้งค่า `GARUDUST_LINE_PORT` (ค่าเริ่มต้น `3002`) และกำหนด Webhook URL ใน LINE console เป็น `https://your-host:3002/line`

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

ตั้งค่า key ที่เกี่ยวข้องใน `~/.garudust/.env` แล้วเปลี่ยนโมเดลด้วย `garudust model` หรือตั้งค่า `GARUDUST_MODEL`

---

## เครื่องมือในตัว

| เครื่องมือ | คำอธิบาย |
|-----------|----------|
| `web_fetch` | ดึงข้อมูลจาก URL (หน้าสแตติก) |
| `web_search` | ค้นหาผ่าน Brave Search API (`BRAVE_SEARCH_API_KEY`) |
| `browser` | ควบคุม Chrome/Chromium ผ่าน CDP — navigate, คลิก, พิมพ์, screenshot, รัน JS |
| `read_file` | อ่านไฟล์จากระบบไฟล์ |
| `write_file` | เขียนไฟล์ไปยังระบบไฟล์; credential path ที่ละเอียดอ่อนถูกบล็อคเสมอ |
| `terminal` | รันคำสั่ง shell; ทำงานใน Docker sandbox เมื่อตั้งค่า `terminal_sandbox: docker` |
| `memory` | หน่วยความจำถาวรแบบ key-value (add / read / replace / remove) |
| `user_profile` | อ่านและอัปเดต user profile ที่ถาวร |
| `session_search` | ค้นหาแบบ full-text ข้ามการสนทนาในอดีต (SQLite FTS5) |
| `delegate_task` | สร้าง sub-agent แบบขนานสำหรับงานที่แบ่งย่อย |
| `skills_list` | แสดงรายการสกิลที่มีอยู่ |
| `skill_view` | โหลดคำแนะนำเต็มของสกิลตามชื่อ |
| `write_skill` | สร้างหรืออัปเดตสกิลใน `~/.garudust/skills/` |

**MCP tools** — เชื่อมต่อ [MCP](https://modelcontextprotocol.io) server ใด ๆ โดยเพิ่มในรายการ `mcp_servers` ใน `config.yaml` (ดูที่หัวข้อการตั้งค่า)

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
  garudust-platforms   Telegram, Discord, Slack, Matrix, LINE, Webhook
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
- **แพลตฟอร์มใหม่** — implement `PlatformAdapter` (เช่น Signal, WhatsApp)
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

---

## ใบอนุญาต

MIT — ดูที่ [LICENSE](../../../LICENSE)
