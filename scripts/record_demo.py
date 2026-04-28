#!/usr/bin/env python3
"""Simulate a garudust terminal demo session for termtosvg recording."""

import sys
import time

RESET  = "\033[0m"
BOLD   = "\033[1m"
DIM    = "\033[2m"
GREEN  = "\033[32m"
CYAN   = "\033[36m"
YELLOW = "\033[33m"
BLUE   = "\033[34m"
GRAY   = "\033[90m"

def out(text: str) -> None:
    sys.stdout.write(text)
    sys.stdout.flush()

def type_line(text: str, speed: float = 0.045) -> None:
    for ch in text:
        out(ch)
        time.sleep(speed)

def prompt() -> None:
    out(f"\n{GREEN}❯{RESET} ")
    time.sleep(0.3)

def pause(s: float) -> None:
    time.sleep(s)


# ── Setup ──────────────────────────────────────────────────────────────────────
prompt()
type_line("garudust setup")
pause(0.4)
out("\n")
pause(0.6)

out(f"{BOLD}Garudust Setup{RESET}\n")
out("─" * 48 + "\n")
out("Press Enter to accept the [default] value.\n\n")
out("Setup mode:\n")
out("  1) Quick — provider + model only\n")
out("  2) Full  — provider, model, and platform adapters\n")
out(f"  Choose mode {DIM}[1]{RESET}: ")
pause(0.5)
out("1\n\n")
pause(0.3)

out("LLM Provider:\n")
out(f"  1) ollama      — local Ollama, no API key needed {GREEN}✓ detected{RESET}\n")
out("  2) openrouter  — 200+ hosted models (openrouter.ai)\n")
out("  3) anthropic   — Claude directly\n")
out("  4) vllm        — self-hosted vLLM server\n")
out("  5) custom      — any OpenAI-compatible endpoint\n")
out(f"  Choose provider {DIM}[1]{RESET}: ")
pause(0.5)
out("1\n\n")
pause(0.3)

out(f"  OLLAMA_BASE_URL {DIM}[http://localhost:11434]{RESET}: ")
pause(0.4)
out("\n\n")
pause(0.3)

out(f"  Model {DIM}[llama3.2]{RESET}: ")
pause(0.4)
out("\n\n")
pause(0.8)

out(f"Configuration saved to {DIM}/Users/demo/.garudust{RESET}\n\n")
out(f"{GREEN}✓{RESET} Ollama reachable at http://localhost:11434\n")
out(f"{GREEN}✓{RESET} Model llama3.2 available\n")
out(f"{GREEN}✓{RESET} Ready\n")
pause(1.0)

# ── One-shot ───────────────────────────────────────────────────────────────────
prompt()
type_line('garudust "fastest way to reverse a string in Rust?"')
pause(0.4)
out("\n")
pause(1.2)

out(f"{GRAY}thinking…{RESET}\n\n")
pause(1.0)

response1 = (
    "The fastest way to reverse a string in Rust is:\n\n"
    f"  {CYAN}let reversed: String = s.chars().rev().collect();{RESET}\n\n"
    "For byte strings (ASCII only), `s.bytes().rev()` is slightly faster.\n"
    "If performance is critical, reverse in-place on a `Vec<u8>` and convert back.\n"
)
for ch in response1:
    out(ch)
    time.sleep(0.008)
pause(1.2)

# ── TUI ────────────────────────────────────────────────────────────────────────
prompt()
type_line("garudust")
pause(0.4)
out("\n")
pause(1.0)

out(f"\n{BOLD}Garudust{RESET}  {DIM}llama3.2 · /help for commands · Ctrl+C to quit{RESET}\n\n")
pause(0.6)

out(f"{BLUE}You{RESET}  ")
type_line("what makes Rust memory-safe without a garbage collector?", speed=0.032)
out("\n")
pause(1.5)

out(f"{GRAY}thinking…{RESET}\n\n")
pause(1.2)

out(f"{GREEN}Garudust{RESET}  ")
response2 = (
    "Rust achieves memory safety through three mechanisms:\n\n"
    f"  {BOLD}1. Ownership{RESET} — every value has exactly one owner; it's freed when\n"
    "     the owner goes out of scope.\n\n"
    f"  {BOLD}2. Borrowing{RESET} — references can be shared (&T) or exclusive (&mut T),\n"
    "     never both at once, enforced at compile time.\n\n"
    f"  {BOLD}3. Lifetimes{RESET} — the compiler tracks how long references are valid,\n"
    "     eliminating dangling pointers without runtime overhead.\n\n"
    "The result: no use-after-free, no data races, zero runtime cost.\n"
)
for ch in response2:
    out(ch)
    time.sleep(0.007)
pause(1.0)

out(f"\n{BLUE}You{RESET}  ")
type_line("/new", speed=0.08)
out("\n")
pause(0.4)
out(f"{GRAY}Session cleared.{RESET}\n\n")
pause(0.5)

out(f"{BLUE}You{RESET}  ")
type_line("write a haiku about zero-cost abstractions", speed=0.035)
out("\n")
pause(1.2)

out(f"{GRAY}thinking…{RESET}\n\n")
pause(1.0)

out(f"{GREEN}Garudust{RESET}  ")
haiku = (
    "High-level code blooms —\n"
    "  no runtime cost beneath it,\n"
    "  iron runs as thought.\n"
)
for ch in haiku:
    out(ch)
    time.sleep(0.03)
pause(1.5)

out(f"\n{DIM}^C{RESET}\n")
pause(0.3)
prompt()
out("\n")
