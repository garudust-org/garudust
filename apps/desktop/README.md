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
# generate icons (see the Icons note below) — required before the first run
cargo build --release -p garudust-server   # build the gateway…
npm run --prefix apps/desktop stage-sidecar # …and stage it as the sidecar

# run (Vite dev server + Tauri window with hot reload)
npm run --prefix apps/desktop dev
```

## Build installers

```bash
cargo build --release -p garudust-server
npm run --prefix apps/desktop stage-sidecar
npm run --prefix apps/desktop build   # → src-tauri/target/release/bundle/{dmg,nsis,appimage,deb}
```

> **Icons:** the first `dev` or `build` requires app icons (the Tauri build
> script validates them). Generate them once from a **square** PNG — note
> `tauri icon` rejects non-square sources, so `logo-agent.jpg` must be squared
> first:
>
> ```bash
> # macOS: make a 1024×1024 PNG, then generate all icon sizes
> sips -s format png -z 1024 1024 ../../assets/logo-agent.jpg --out /tmp/icon.png
> npm run tauri icon /tmp/icon.png   # writes src-tauri/icons/ (gitignored)
> ```

## Notes

- The desktop sidecar uses the plain `garudust-server` (no `web-ui` feature) —
  Tauri serves the SPA, the gateway only serves the API.
- `src-tauri/binaries/` and `src-tauri/gen/` are build artifacts (gitignored).
