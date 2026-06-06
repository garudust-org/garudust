# Garudust Desktop

A thin [Tauri 2](https://tauri.app) shell that wraps the web dashboard (`../../web`,
a Rust/Leptos WASM app) and runs `garudust-server` as a local sidecar. Chosen
over Electron to keep the app small (OS webview, no bundled Chromium/Node) in
line with Garudust's single-small-binary philosophy.

**No Node/npm** — the UI is Rust (Leptos → WASM, built with Trunk) and the build
uses the Rust `cargo tauri` CLI.

## How it works

```
Tauri window  ──loads──▶  web/dist  (Leptos WASM SPA, bundled by Tauri)
     │
     └─ on launch: spawns garudust-server on 127.0.0.1:<free port>,
        injects window.__GARUDUST_GATEWAY__ = "http://127.0.0.1:<port>"
        so the SPA's API client targets the sidecar (not tauri://).
```

The sidecar binds loopback only, so the desktop app never exposes the agent to
the network. It is killed when the window closes.

## Prerequisites (one-time)

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk --locked        # builds the Leptos SPA
cargo install tauri-cli --locked    # the `cargo tauri` command
```

## Develop

```bash
cargo build --release -p garudust-server   # build the gateway…
apps/desktop/scripts/stage-sidecar.sh      # …and stage it as the sidecar

# run (Trunk dev server + Tauri window, hot-reloads on .rs changes)
cd apps/desktop && cargo tauri dev
```

## Build installers

CI builds and attaches `.dmg` / `.exe` / `.AppImage` / `.deb` to every GitHub
Release (the `desktop` job in `.github/workflows/release.yml`, triggered on `v*`
tags). To build locally:

```bash
cargo build --release -p garudust-server
apps/desktop/scripts/stage-sidecar.sh
cd apps/desktop && cargo tauri build   # → src-tauri/target/release/bundle/{dmg,nsis,appimage,deb}
```

> **Icons:** committed under `src-tauri/icons/`, generated from the square
> source `assets/icon.png`. To rebrand, replace that source (1024×1024 PNG) and
> regenerate: `cargo tauri icon ../../assets/icon.png`.

## Code signing (optional)

Builds are unsigned by default, so macOS Gatekeeper and Windows SmartScreen warn
on first launch. To ship signed + notarized macOS builds, add these repo secrets
— the release workflow passes them to `tauri-action` automatically (absent =
unsigned, no failure):

`APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`,
`APPLE_ID`, `APPLE_PASSWORD` (app-specific password), `APPLE_TEAM_ID`.

## Notes

- The desktop sidecar uses the plain `garudust-server` (no `web-ui` feature) —
  Tauri serves the SPA, the gateway only serves the API. It binds `127.0.0.1`
  (the shell passes `--host 127.0.0.1`) so it is never exposed to the network.
- `src-tauri/binaries/` and `src-tauri/gen/` are build artifacts (gitignored);
  `src-tauri/icons/` is committed.
