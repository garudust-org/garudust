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
- **เปลี่ยน LLM ด้วย env var เดียว** — รองรับ Anthropic, OpenRouter, AWS Bedrock, Ollama, vLLM, ThaiLLM
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
garudust         # เปิด TUI แบบ interactive
garudust "สรุป git log 7 วันล่าสุดเป็น changelog"   # one-shot
```

### รันเป็น server

```bash
garudust-server --port 3000
```

เปิดใช้แพลตฟอร์มโดยตั้ง token ใน `~/.garudust/.env` และ enable ใน `~/.garudust/config.yaml`:

```yaml
platforms:
  line:
    enabled: true
    port: 3002
    webhook_path: /line
```

<div align="center">
  <img src="../../../assets/demo-line.jpg" alt="LINE Demo" width="420"/>
</div>

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
| OpenRouter | `provider: openrouter` *(default)* | `OPENROUTER_API_KEY` |
| AWS Bedrock | `provider: bedrock` | `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` |
| Ollama | `provider: ollama` + `base_url` | *(ไม่ต้องการ)* |
| vLLM | `provider: vllm` + `base_url` | `VLLM_API_KEY` |
| ThaiLLM | `provider: thaillm` | `THAILLM_API_KEY` |
| OpenAI-compat อื่น ๆ | `provider: custom` + `base_url` | API key ที่เกี่ยวข้อง |

Fallback keys: `LLM_FALLBACK_API_KEYS=key2,key3` — สลับอัตโนมัติเมื่อ auth ล้มเหลว

---

## Skills และ Memory

agent บันทึกทุกสิ่งที่เรียนรู้ไว้ใน `~/.garudust/memory/` และโหลดทุก session workflow ที่ใช้ซ้ำได้จะถูกเขียนเป็น skill ใน `~/.garudust/skills/` โดยอัตโนมัติ

ติดตั้ง skill จาก [agentskills.io](https://agentskills.io):

```bash
garudust skill install agentskills-org/hub/git-workflow
garudust tool install weather   # เครื่องมือจากชุมชน
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
