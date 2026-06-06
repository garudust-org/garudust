# Garudust Desktop (native)

A **pure-Rust native** desktop app built with [egui/eframe](https://github.com/emilk/egui)
— no webview, WASM, JavaScript, HTTP sidecar, or npm. The agent is embedded
**in-process**, so a single binary launches instantly and talks to the agent by
direct function calls.

```
egui window (native, GPU)
   ↕ channels
Tokio thread  ──owns──▶  garudust-agent (in-process)
```

## Features

Full parity with the web dashboard:

- **Chat** — streaming responses, runtime model picker (routing hints), New chat,
  Stop (aborts the in-flight run), markdown rendering.
- **Status** — model / provider / sandbox / approval mode.
- **Config** — provider dropdown (auto-fills a default model) with an API-key
  hint, approval-mode / sandbox selects, numeric settings, and a routing editor.
  Saving writes `config.yaml` and **rebuilds the agent in place** (no restart).
- **Secrets** — write-only, masked `.env` editor with delete.
- **Appearance** — Dark/Light theme and 3 font sizes, persisted across launches
  (along with window geometry).

Config/Secrets read & write `~/.garudust/config.yaml` and `.env` directly.

## Run

```bash
cargo run -p garudust-desktop-native --release
# or from this dir:
cd apps/desktop-native && cargo run --release
```

No prerequisites on macOS/Windows. On Linux, install the usual egui build deps
(`libxkbcommon-dev`, `libwayland-dev`, `libgl1-mesa-dev`, …).

## Notes

- Uses the same `~/.garudust/` config as the CLI and server.
- Setting a secret writes `.env`; restart to pick up new API keys (the in-memory
  secret map is loaded once at startup).
- App icon / Thai font: a macOS Thai system font is loaded as a fallback so Thai
  text renders; the logo banner is embedded from `assets/logo-agent.jpg`.
