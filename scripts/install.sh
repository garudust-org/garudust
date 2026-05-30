#!/usr/bin/env sh
# Garudust installer — macOS & Linux (all archs, incl. ARM / Raspberry Pi / WSL).
#
#   curl -fsSL https://raw.githubusercontent.com/garudust-org/garudust-agent/main/scripts/install.sh | sh
#
# Environment overrides:
#   GARUDUST_VERSION   pin a release tag (e.g. v0.13.1); default: latest
#   GARUDUST_BIN_DIR   install destination; default: /usr/local/bin (or ~/.local/bin)
#
# POSIX sh — no bashisms, so it runs under sh, bash, dash, zsh, busybox.
set -eu

REPO="garudust-org/garudust-agent"

info()  { printf '\033[1;34m==>\033[0m %s\n' "$1"; }
warn()  { printf '\033[1;33mwarning:\033[0m %s\n' "$1" >&2; }
die()   { printf '\033[1;31merror:\033[0m %s\n' "$1" >&2; exit 1; }
have()  { command -v "$1" >/dev/null 2>&1; }

# ── Detect platform ──────────────────────────────────────────────────────────
os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Darwin) os_name="apple-darwin" ;;
  Linux)  os_name="unknown-linux-musl" ;;
  *)      die "unsupported OS '$os'. On Windows use the PowerShell installer (install.ps1)." ;;
esac

case "$arch" in
  x86_64 | amd64)  cpu="x86_64" ;;
  arm64 | aarch64) cpu="aarch64" ;;
  *)               die "unsupported architecture '$arch'." ;;
esac

target="${cpu}-${os_name}"

# ── Resolve version ──────────────────────────────────────────────────────────
have curl || die "curl is required."
have tar  || die "tar is required."

version="${GARUDUST_VERSION:-}"
if [ -z "$version" ]; then
  info "Resolving latest release…"
  version="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' | head -1 | cut -d'"' -f4)"
  [ -n "$version" ] || die "could not determine the latest version. Set GARUDUST_VERSION."
fi

asset="garudust-${version}-${target}.tar.gz"
url="https://github.com/${REPO}/releases/download/${version}/${asset}"

# ── Download ─────────────────────────────────────────────────────────────────
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM

info "Downloading ${asset}…"
curl -fSL --progress-bar "$url" -o "$tmp/$asset" \
  || die "download failed: $url (does a release exist for ${target}?)"

# ── Verify checksum (best-effort) ────────────────────────────────────────────
sums_url="https://github.com/${REPO}/releases/download/${version}/SHA256SUMS.txt"
if curl -fsSL "$sums_url" -o "$tmp/SHA256SUMS.txt" 2>/dev/null; then
  expected="$(grep " ${asset}\$" "$tmp/SHA256SUMS.txt" | awk '{print $1}' | head -1)"
  if [ -n "$expected" ]; then
    if have sha256sum;  then actual="$(sha256sum "$tmp/$asset" | awk '{print $1}')"
    elif have shasum;   then actual="$(shasum -a 256 "$tmp/$asset" | awk '{print $1}')"
    else actual=""; warn "no sha256 tool found; skipping checksum verification."
    fi
    if [ -n "$actual" ] && [ "$actual" != "$expected" ]; then
      die "checksum mismatch for ${asset} (expected ${expected}, got ${actual})."
    fi
    [ -n "$actual" ] && info "Checksum verified."
  fi
else
  warn "SHA256SUMS.txt not found for ${version}; skipping checksum verification."
fi

# ── Extract ──────────────────────────────────────────────────────────────────
tar -xzf "$tmp/$asset" -C "$tmp"
src="$tmp/garudust-${version}-${target}"
[ -f "$src/garudust" ] || die "archive layout unexpected: $src/garudust not found."

# ── Choose install dir ───────────────────────────────────────────────────────
sudo=""
bin_dir="${GARUDUST_BIN_DIR:-}"
if [ -z "$bin_dir" ]; then
  if [ -w /usr/local/bin ] 2>/dev/null; then
    bin_dir="/usr/local/bin"
  elif have sudo; then
    bin_dir="/usr/local/bin"; sudo="sudo"
  else
    bin_dir="$HOME/.local/bin"
  fi
fi

if [ "$sudo" = "sudo" ]; then
  info "Installing to ${bin_dir} (requires sudo)…"
else
  info "Installing to ${bin_dir}…"
fi

$sudo mkdir -p "$bin_dir"
for b in garudust garudust-server; do
  $sudo install -m 0755 "$src/$b" "$bin_dir/$b"
done

# ── Done ─────────────────────────────────────────────────────────────────────
info "Installed garudust ${version} → ${bin_dir}"
case ":${PATH}:" in
  *":${bin_dir}:"*) ;;
  *) warn "${bin_dir} is not on your PATH. Add it, e.g.:"
     printf '       export PATH="%s:$PATH"\n' "$bin_dir" ;;
esac

if have garudust; then
  printf '\n'
  garudust --version 2>/dev/null || true
  info "Run 'garudust setup' to get started."
fi
