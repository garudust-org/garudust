#!/usr/bin/env bash
# Stage the release `garudust-server` binary as a Tauri sidecar.
#
# Tauri's `externalBin` resolves `binaries/garudust-server` to a file suffixed
# with the host target triple (e.g. `garudust-server-aarch64-apple-darwin`).
# This copies the workspace release build into that location. The desktop
# sidecar only serves the gateway API (Tauri serves the SPA), so the plain
# `garudust-server` — no `web-ui` feature — is sufficient.
#
# Build it first:  cargo build --release -p garudust-server
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
repo_root="$(cd "$here/../../.." && pwd)"

triple="$(rustc -vV | sed -n 's/^host: //p')"
if [ -z "$triple" ]; then
  echo "could not determine host target triple from 'rustc -vV'" >&2
  exit 1
fi

ext=""
case "$triple" in *windows*) ext=".exe" ;; esac

src="$repo_root/target/release/garudust-server$ext"
dest_dir="$here/../src-tauri/binaries"
dest="$dest_dir/garudust-server-$triple$ext"

mkdir -p "$dest_dir"
cp "$src" "$dest"
echo "staged $src"
echo "     -> $dest"
