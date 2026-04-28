#!/usr/bin/env python3
"""Generate an asciinema v2 cast file for the Garudust demo."""

import json, sys

W, H = 110, 36
events = []
t = 0.0

def e(delay: float, text: str) -> None:
    global t
    t += delay
    events.append([round(t, 3), "o", text])

def typing(text: str, speed: float = 0.05) -> None:
    for ch in text:
        e(speed, ch)

def nl() -> None:
    e(0.05, "\r\n")

RESET = "\033[0m"; BOLD = "\033[1m"; DIM = "\033[2m"
GREEN = "\033[32m"; CYAN = "\033[36m"; GRAY = "\033[90m"; BLUE = "\033[34m"

# ── prompt helper ──────────────────────────────────────────────────────────────
def prompt(cmd: str = "") -> None:
    e(0.3, f"\r\n{GREEN}❯{RESET} ")
    if cmd:
        typing(cmd)
        e(0.3, "\r\n")

# ── Setup ──────────────────────────────────────────────────────────────────────
e(0.5, f"{GREEN}❯{RESET} ")
typing("garudust setup")
e(0.4, "\r\n")
e(0.8, f"{BOLD}Garudust Setup{RESET}\r\n")
e(0.1, "─" * 48 + "\r\n")
e(0.1, "Press Enter to accept the [default] value.\r\n\r\n")
e(0.3, "Setup mode:\r\n")
e(0.1, "  1) Quick — provider + model only\r\n")
e(0.1, "  2) Full  — provider, model, and platform adapters\r\n")
e(0.5, f"  Choose mode {DIM}[1]{RESET}: ")
e(0.6, "1\r\n\r\n")

e(0.3, "LLM Provider:\r\n")
e(0.1, f"  1) ollama      — local Ollama, no API key needed  {GREEN}✓ detected{RESET}\r\n")
e(0.1, "  2) openrouter  — 200+ hosted models (openrouter.ai)\r\n")
e(0.1, "  3) anthropic   — Claude directly\r\n")
e(0.1, "  4) vllm        — self-hosted vLLM server\r\n")
e(0.1, "  5) custom      — any OpenAI-compatible endpoint\r\n")
e(0.5, f"  Choose provider {DIM}[1]{RESET}: ")
e(0.6, "1\r\n\r\n")

e(0.3, f"  OLLAMA_BASE_URL {DIM}[http://localhost:11434]{RESET}: ")
e(0.5, "\r\n\r\n")
e(0.3, f"  Model {DIM}[llama3.2]{RESET}: ")
e(0.5, "\r\n\r\n")
e(0.5, f"Configuration saved to {DIM}/Users/demo/.garudust{RESET}\r\n\r\n")
e(0.2, f"{GREEN}✓{RESET} Ollama reachable at http://localhost:11434\r\n")
e(0.2, f"{GREEN}✓{RESET} Model llama3.2 available\r\n")
e(0.2, f"{GREEN}✓{RESET} Ready\r\n")

# ── One-shot ───────────────────────────────────────────────────────────────────
prompt('garudust "fastest way to reverse a string in Rust?"')
e(1.0, f"{GRAY}thinking…{RESET}\r\n\r\n")
e(0.8, "The fastest way to reverse a string in Rust:\r\n\r\n")
e(0.2, f"  {CYAN}let reversed: String = s.chars().rev().collect();{RESET}\r\n\r\n")
e(0.3, "For ASCII-only, `s.bytes().rev()` is slightly faster.\r\n")
e(0.3, "In-place: reverse a `Vec<u8>` then convert back.\r\n")

# ── TUI ────────────────────────────────────────────────────────────────────────
prompt("garudust")
e(0.8, f"\r\n{BOLD}Garudust{RESET}  {DIM}llama3.2 · /help for commands · Ctrl+C to quit{RESET}\r\n\r\n")

e(0.5, f"{BLUE}You{RESET}  ")
typing("what makes Rust memory-safe without a garbage collector?", speed=0.03)
e(0.3, "\r\n")
e(1.2, f"{GRAY}thinking…{RESET}\r\n\r\n")
e(0.5, f"{GREEN}Garudust{RESET}  Rust achieves memory safety through three mechanisms:\r\n\r\n")
e(0.3, f"  {BOLD}1. Ownership{RESET}  — one owner per value; freed when owner goes out of scope.\r\n")
e(0.3, f"  {BOLD}2. Borrowing{RESET}  — shared (&T) or exclusive (&mut T), never both at once.\r\n")
e(0.3, f"  {BOLD}3. Lifetimes{RESET}  — compiler tracks reference validity, no dangling pointers.\r\n\r\n")
e(0.3, "Result: no use-after-free, no data races, zero runtime cost.\r\n")

e(0.8, f"\r\n{BLUE}You{RESET}  ")
typing("/new", speed=0.08)
e(0.3, "\r\n")
e(0.5, f"{GRAY}Session cleared.{RESET}\r\n\r\n")

e(0.4, f"{BLUE}You{RESET}  ")
typing("write a haiku about zero-cost abstractions", speed=0.035)
e(0.3, "\r\n")
e(1.2, f"{GRAY}thinking…{RESET}\r\n\r\n")
e(0.5, f"{GREEN}Garudust{RESET}  High-level code blooms —\r\n")
e(0.4, "           no runtime cost beneath it,\r\n")
e(0.4, "           iron runs as thought.\r\n")

e(1.5, f"\r\n{DIM}^C{RESET}\r\n")
e(0.3, f"{GREEN}❯{RESET} \r\n")

# ── Write cast file ────────────────────────────────────────────────────────────
out = sys.argv[1] if len(sys.argv) > 1 else "/tmp/demo.cast"
with open(out, "w") as f:
    f.write(json.dumps({"version": 2, "width": W, "height": H}) + "\n")
    for ev in events:
        f.write(json.dumps(ev) + "\n")

print(f"wrote {len(events)} events to {out}  (duration {t:.1f}s)")
