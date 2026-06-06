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

## เริ่มใช้งาน

**01 — ติดตั้ง**

```bash
curl -fsSL https://raw.githubusercontent.com/garudust-org/garudust-agent/main/scripts/install.sh | sh
```

macOS & Linux ทุก arch (ARM, Raspberry Pi, WSL) — Windows: `irm .../scripts/install.ps1 | iex` ปรับแต่งด้วย `GARUDUST_VERSION` / `GARUDUST_BIN_DIR`

<details>
<summary>ดาวน์โหลดเองหรือ build จาก source</summary>

ดาวน์โหลด binary สำเร็จรูปจาก [GitHub Releases](https://github.com/garudust-org/garudust-agent/releases/latest):

| OS | สถาปัตยกรรม | ไฟล์ |
|----|------------|------|
| macOS | Apple Silicon (M1/M2/M3/M4) | `garudust-*-aarch64-apple-darwin.tar.gz` |
| macOS | Intel | `garudust-*-x86_64-apple-darwin.tar.gz` |
| Linux | x86_64 | `garudust-*-x86_64-unknown-linux-musl.tar.gz` |
| Linux | ARM64 (Raspberry Pi 4/5, Jetson) | `garudust-*-aarch64-unknown-linux-musl.tar.gz` |
| Windows | x86_64 | `garudust-*-x86_64-pc-windows-msvc.zip` |

หรือ build จาก source (Rust 1.87+): `git clone https://github.com/garudust-org/garudust-agent && cargo build --release`

</details>

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

# คำสั่งย่อยสำหรับจัดการ
garudust setup                     # ตัวช่วยตั้งค่าครั้งแรกแบบโต้ตอบ
garudust doctor                    # ตรวจสอบ environment และ config
garudust config show               # ดู config ปัจจุบัน
garudust config set <key> <value> # ตั้งค่า config
garudust model [<name>]            # ดูหรือเปลี่ยน model ที่ใช้งาน
# Script tool
garudust tool list                 # แสดง tool ที่ติดตั้งแล้ว + ที่มีใน hub
garudust tool install <name>       # ติดตั้ง tool จาก hub
garudust tool uninstall <name>     # ลบ tool ที่ติดตั้งไว้
garudust tool update [<name>]      # อัปเดต tool (ละชื่อ = อัปเดตทั้งหมด)

# Skill
garudust skill list                # แสดง skill ที่ติดตั้งแล้ว + ที่มีใน hub
garudust skill install <source>    # ติดตั้งจาก hub / GitHub / URL / well-known
garudust skill uninstall <name>    # ลบ skill ที่ติดตั้งไว้
garudust skill update [<name>]     # อัปเดต skill (ละชื่อ = อัปเดตทั้งหมด)
garudust skill validate [<path>]   # ตรวจสอบ frontmatter ของ SKILL.md
```


---

## ทำไมต้อง Garudust?

- **~10 MB, cold start < 20 ms** — ไฟล์เดียว ไม่ต้องพึ่ง runtime อื่น
- **พัฒนาตัวเองได้** — จดจำความชอบ สร้าง skill จาก workflow และแก้ไขตัวเองโดยไม่ต้องบอกซ้ำ
- **รัน tool พร้อมกัน** — tool ที่ไม่ขัดแย้งกันรันคู่ขนานอัตโนมัติ
- **24 LLM provider** — Anthropic, OpenAI, Gemini, Groq, Ollama, Bedrock และอีกมาก — เปลี่ยนด้วยบรรทัดเดียว
- **7 แพลตฟอร์มในกระบวนการเดียว** — Telegram, Discord, Slack, Matrix, LINE, WhatsApp, Webhook
- **ปลอดภัยตั้งแต่ต้น** — sandbox 3 รูปแบบ (host, Docker, SSH remote), RBAC, per-user rate limit, redact secret อัตโนมัติ

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
  terminal_sandbox: none     # none | docker | ssh
  rate_limit_rpm: ~          # จำกัดต่อ IP (~ = ไม่จำกัด)
  rate_limit_rpm_per_user: ~ # จำกัดต่อ (platform, user_id)

  # ── SSH sandbox (terminal_sandbox: ssh) ──────────────────────────────
  # ssh_host: "192.168.1.50"              # จำเป็น
  # ssh_user: "pi"                        # optional — ค่าเริ่มต้นคือ user ปัจจุบัน
  # ssh_port: 22                          # optional — ค่าเริ่มต้น 22
  # ssh_key_path: ~/.ssh/garudust_pi      # optional — ใช้ ~/.ssh/id_* ถ้าไม่ระบุ
  # ssh_jump_host: "bastion.example.com"  # optional — ProxyJump สำหรับ host ที่อยู่หลัง NAT
  # ssh_remote_cwd: "/home/pi/scripts"    # optional — ต้องเป็น absolute path ไม่มี metacharacter
  # ssh_options: ["IdentitiesOnly=yes"]   # optional — extra -o flags

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

> **Production:** ตั้ง `terminal_sandbox: docker` (local container) หรือ `terminal_sandbox: ssh` (remote host) เพื่อ sandbox shell execution และ `max_delegation_depth: 0` เพื่อป้องกัน sub-agent chain

> **หมายเหตุ:** การตั้ง `platform.session_per_user: false` ทำให้ผู้ใช้ทุกคนแชร์ context การสนทนาเดียวกัน server จะ log `WARN` ตอน startup เพื่อเตือน เหมาะสำหรับ deployment แบบผู้ใช้คนเดียวเท่านั้น

---

## Terminal Sandbox

`terminal` tool รองรับ 3 backend:

| โหมด | `terminal_sandbox` | รันที่ไหน | ต้องการ |
|---|---|---|---|
| โฮสต์โดยตรง | `none` | เครื่องของคุณ | ไม่ต้องการ |
| Docker container | `docker` | container ที่แยกออกมา | Docker daemon |
| Remote SSH host | `ssh` | host ที่มี sshd | SSH key auth |

ทุกโหมดใช้ hardline blocks เดียวกัน (fork bomb, `rm -rf /`, `mkfs` ฯลฯ) และ approval gate เดียวกัน — sandbox ควบคุมเฉพาะ *ที่ที่* command รัน

### SSH Sandbox

command ถูกส่งผ่าน `ssh` binary ของระบบไปยัง remote host เหมาะสำหรับจัดการ remote server, Raspberry Pi หรือเครื่อง build โดยไม่ต้องเปิด port ให้อินเทอร์เน็ต — agent SSH ออกไป remote host ต้องมีแค่ port 22 เปิดอยู่

**Config fields** (ทั้งหมดอยู่ภายใต้ `security:` ใน `config.yaml`):

| Field | ประเภท | ค่าเริ่มต้น | คำอธิบาย |
|---|---|---|---|
| `ssh_host` | string | — | **จำเป็น.** hostname หรือ IP ของ remote |
| `ssh_user` | string | user ปัจจุบัน | ชื่อผู้ใช้สำหรับ login |
| `ssh_port` | integer | `22` | SSH port |
| `ssh_key_path` | path | `~/.ssh/id_*` | ไฟล์ private key |
| `ssh_jump_host` | string | — | ProxyJump bastion (`user@host:port`) สำหรับ host ที่อยู่หลัง NAT |
| `ssh_remote_cwd` | string | — | นำหน้าทุก command ด้วย `cd <dir> &&`; ต้องเป็น absolute path ที่ไม่มี shell metacharacter (เช่น `/home/pi/scripts`) |
| `ssh_options` | list | `[]` | `-o key=value` flags เพิ่มเติม (ต่อท้าย hardened defaults) |

**Environment variable overrides** — ไม่ต้องใช้ config.yaml:

```bash
GARUDUST_TERMINAL_SANDBOX=ssh
GARUDUST_SSH_HOST=192.168.1.50
GARUDUST_SSH_USER=pi
GARUDUST_SSH_PORT=22
GARUDUST_SSH_KEY_PATH=/home/user/.ssh/garudust_pi
```

**ตัวอย่างขั้นต่ำ** — Raspberry Pi ที่อยู่หลัง home router:

```yaml
security:
  terminal_sandbox: ssh
  ssh_host: "192.168.1.50"
  ssh_user: "pi"
  ssh_key_path: ~/.ssh/garudust_pi
```

**ผ่าน bastion** — Pi เข้าถึงได้ผ่าน jump server สาธารณะเท่านั้น:

```yaml
security:
  terminal_sandbox: ssh
  ssh_host: "pi.internal"
  ssh_user: "pi"
  ssh_key_path: ~/.ssh/garudust_pi
  ssh_jump_host: "bastion.example.com"
```

**คุณสมบัติความปลอดภัยที่ใช้งานอัตโนมัติ:**

- `BatchMode=yes` — ไม่มี interactive prompt; ล้มเหลวทันทีถ้า key auth ถูกปฏิเสธ
- `StrictHostKeyChecking=accept-new` — เชื่อถือ host ครั้งแรกอัตโนมัติ, ปฏิเสธ host key ที่เปลี่ยนแปลง (ป้องกัน MITM)
- `ConnectTimeout` จำกัดไว้ที่ 30 วินาที — ไม่มีการค้างแบบไม่มีกำหนด
- `ServerAliveInterval=10 ServerAliveCountMax=3` — ตรวจพบการเชื่อมต่อที่ตายแล้วใน ~30 วินาที
- `--` ก่อน command — ป้องกัน command ที่ขึ้นต้นด้วย `-` ถูกตีความเป็น SSH flag
- `env_clear()` ก่อน spawn `ssh` — API key และ secret ไม่ถูกส่งไปยัง remote host
- `ssh_remote_cwd` ได้รับการตรวจสอบว่าเป็น absolute path ที่ปลอดภัย — ค่าที่มี shell metacharacter จะถูกปฏิเสธก่อนส่ง command
- `ssh_options` ถูกต่อท้าย *หลัง* hardened defaults — ไม่สามารถ override `BatchMode` หรือ `StrictHostKeyChecking` ได้

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
git config core.hooksPath .githooks   # เปิดใช้ pre-push checks (fmt + tests)
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
