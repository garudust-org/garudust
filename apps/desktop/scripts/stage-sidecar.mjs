// Stage the release `garudust-server` binary as a Tauri sidecar.
//
// Tauri's `externalBin` resolves `binaries/garudust-server` to a file suffixed
// with the host target triple (e.g. `garudust-server-aarch64-apple-darwin`).
// This copies the workspace release build into that location. The desktop
// sidecar only serves the gateway API (the SPA is bundled by Tauri itself), so
// the plain `garudust-server` — no `web-ui` feature — is sufficient.
//
// Build it first:  cargo build --release -p garudust-server

import { execSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(here, "..", "..", ".."); // apps/desktop/scripts -> repo root
const binariesDir = join(here, "..", "src-tauri", "binaries");

const hostLine = execSync("rustc -vV").toString();
const triple = hostLine.match(/host:\s*(\S+)/)?.[1];
if (!triple) {
  console.error("could not determine host target triple from `rustc -vV`");
  process.exit(1);
}

const ext = process.platform === "win32" ? ".exe" : "";
const src = join(repoRoot, "target", "release", `garudust-server${ext}`);
const dest = join(binariesDir, `garudust-server-${triple}${ext}`);

mkdirSync(binariesDir, { recursive: true });
copyFileSync(src, dest);
console.log(`staged ${src}\n     -> ${dest}`);
