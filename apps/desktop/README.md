# Garudust Desktop

A thin [Tauri 2](https://tauri.app) shell that wraps the web dashboard (`../../web`)
and runs `garudust-server` as a local sidecar. Chosen over Electron to keep the
app small (OS webview, no bundled Chromium/Node) in line with Garudust's
single-small-binary philosophy.

## How it works

```
Tauri window  ──loads──▶  web/dist SPA (bundled by Tauri)
     │
     └─ on launch: spawns garudust-server on 127.0.0.1:<free port>,
        injects window.__GARUDUST_GATEWAY__ = "http://127.0.0.1:<port>"
        so the SPA's API client targets the sidecar (not tauri://).
```

The sidecar binds loopback only, so the desktop app never exposes the agent to
the network. It is killed when the window closes.

## Develop

```bash
# one-time
npm install --prefix apps/desktop

# build the gateway and stage it as the sidecar
cargo build --release -p garudust-server
npm run --prefix apps/desktop stage-sidecar

# run (Vite dev server + Tauri window with hot reload)
npm run --prefix apps/desktop dev
```

## Build installers

```bash
cargo build --release -p garudust-server
npm run --prefix apps/desktop stage-sidecar
npm run --prefix apps/desktop build   # → src-tauri/target/release/bundle/{dmg,nsis,appimage,deb}
```

> **Icons:** first build requires app icons. Generate them once from the logo:
> `npm run --prefix apps/desktop tauri icon ../../assets/logo-agent.jpg`
> (writes `src-tauri/icons/`).

## Notes

- The desktop sidecar uses the plain `garudust-server` (no `web-ui` feature) —
  Tauri serves the SPA, the gateway only serves the API.
- `src-tauri/binaries/` and `src-tauri/gen/` are build artifacts (gitignored).
