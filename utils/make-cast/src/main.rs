use std::{fmt::Write as _, fs, path::PathBuf};

const WIDTH: u32 = 110;
const HEIGHT: u32 = 36;

fn ansi(code: &str) -> String {
    format!("\x1b{code}")
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\r' => out.push_str("\\r"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => write!(out, "\\u{:04x}", c as u32).unwrap(),
            c => out.push(c),
        }
    }
    out
}

struct Cast {
    events: Vec<(f64, String)>,
    t: f64,
}

impl Cast {
    fn new() -> Self {
        Self { events: Vec::new(), t: 0.0 }
    }

    fn e(&mut self, delay: f64, text: &str) {
        self.t += delay;
        let t = (self.t * 1000.0).round() / 1000.0;
        self.events.push((t, text.to_string()));
    }

    fn typing(&mut self, text: &str, speed: f64) {
        for ch in text.chars() {
            self.e(speed, &ch.to_string());
        }
    }

    fn nl(&mut self) {
        self.e(0.05, "\r\n");
    }

    fn prompt(&mut self, cmd: &str) {
        let g = ansi("[32m");
        let r = ansi("[0m");
        self.e(0.3, &format!("\r\n{g}\u{276f}{r} "));
        if !cmd.is_empty() {
            self.typing(cmd, 0.04);
            self.e(0.3, "\r\n");
        }
    }

    fn render(&self) -> String {
        let mut out = String::new();
        writeln!(out, r#"{{"version":2,"width":{WIDTH},"height":{HEIGHT}}}"#).unwrap();
        for (t, text) in &self.events {
            let escaped = json_escape(text);
            writeln!(out, r#"[{t:.3},"o","{escaped}"]"#).unwrap();
        }
        out
    }
}

fn main() {
    let out_path: PathBuf = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/demo.cast".to_string())
        .into();

    let reset  = ansi("[0m");
    let bold   = ansi("[1m");
    let dim    = ansi("[2m");
    let green  = ansi("[32m");
    let cyan   = ansi("[36m");
    let gray   = ansi("[90m");
    let blue   = ansi("[34m");

    let mut c = Cast::new();

    // ── Setup ──────────────────────────────────────────────────────────────────
    c.e(0.5, &format!("{green}\u{276f}{reset} "));
    c.typing("garudust setup", 0.05);
    c.e(0.4, "\r\n");
    c.e(0.8, &format!("{bold}Garudust Setup{reset}\r\n"));
    c.e(0.1, &format!("{}\r\n", "\u{2500}".repeat(48)));
    c.e(0.1, "Press Enter to accept the [default] value.\r\n\r\n");
    c.e(0.3, "Setup mode:\r\n");
    c.e(0.1, "  1) Quick \u{2014} provider + model only\r\n");
    c.e(0.1, "  2) Full  \u{2014} provider, model, and platform adapters\r\n");
    c.e(0.5, &format!("  Choose mode {dim}[1]{reset}: "));
    c.e(0.6, "1\r\n\r\n");

    c.e(0.3, "LLM Provider:\r\n");
    c.e(0.1, &format!("  1) ollama      \u{2014} local Ollama, no API key needed  {green}\u{2713} detected{reset}\r\n"));
    c.e(0.1, "  2) openrouter  \u{2014} 200+ hosted models (openrouter.ai)\r\n");
    c.e(0.1, "  3) anthropic   \u{2014} Claude directly\r\n");
    c.e(0.1, "  4) vllm        \u{2014} self-hosted vLLM server\r\n");
    c.e(0.1, "  5) custom      \u{2014} any OpenAI-compatible endpoint\r\n");
    c.e(0.5, &format!("  Choose provider {dim}[1]{reset}: "));
    c.e(0.6, "1\r\n\r\n");

    c.e(0.3, &format!("  OLLAMA_BASE_URL {dim}[http://localhost:11434]{reset}: "));
    c.e(0.5, "\r\n\r\n");
    c.e(0.3, &format!("  Model {dim}[llama3.2]{reset}: "));
    c.e(0.5, "\r\n\r\n");
    c.e(0.5, &format!("Configuration saved to {dim}/Users/demo/.garudust{reset}\r\n\r\n"));
    c.e(0.2, &format!("{green}\u{2713}{reset} Ollama reachable at http://localhost:11434\r\n"));
    c.e(0.2, &format!("{green}\u{2713}{reset} Model llama3.2 available\r\n"));
    c.e(0.2, &format!("{green}\u{2713}{reset} Ready\r\n"));

    // ── One-shot ───────────────────────────────────────────────────────────────
    c.prompt(r#"garudust "fastest way to reverse a string in Rust?""#);
    c.e(1.0, &format!("{gray}thinking\u{2026}{reset}\r\n\r\n"));
    c.e(0.8, "The fastest way to reverse a string in Rust:\r\n\r\n");
    c.e(0.2, &format!("  {cyan}let reversed: String = s.chars().rev().collect();{reset}\r\n\r\n"));
    c.e(0.3, "For ASCII-only, `s.bytes().rev()` is slightly faster.\r\n");
    c.e(0.3, "In-place: reverse a `Vec<u8>` then convert back.\r\n");

    // ── TUI ────────────────────────────────────────────────────────────────────
    c.prompt("garudust");
    c.e(0.8, &format!("\r\n{bold}Garudust{reset}  {dim}llama3.2 \u{00b7} /help for commands \u{00b7} Ctrl+C to quit{reset}\r\n\r\n"));

    c.e(0.5, &format!("{blue}You{reset}  "));
    c.typing("what makes Rust memory-safe without a garbage collector?", 0.03);
    c.e(0.3, "\r\n");
    c.e(1.2, &format!("{gray}thinking\u{2026}{reset}\r\n\r\n"));
    c.e(0.5, &format!("{green}Garudust{reset}  Rust achieves memory safety through three mechanisms:\r\n\r\n"));
    c.e(0.3, &format!("  {bold}1. Ownership{reset}  \u{2014} one owner per value; freed when owner goes out of scope.\r\n"));
    c.e(0.3, &format!("  {bold}2. Borrowing{reset}  \u{2014} shared (&T) or exclusive (&mut T), never both at once.\r\n"));
    c.e(0.3, &format!("  {bold}3. Lifetimes{reset}  \u{2014} compiler tracks reference validity, no dangling pointers.\r\n\r\n"));
    c.e(0.3, "Result: no use-after-free, no data races, zero runtime cost.\r\n");

    c.e(0.8, &format!("\r\n{blue}You{reset}  "));
    c.typing("/new", 0.08);
    c.e(0.3, "\r\n");
    c.e(0.5, &format!("{gray}Session cleared.{reset}\r\n\r\n"));

    c.e(0.4, &format!("{blue}You{reset}  "));
    c.typing("write a haiku about zero-cost abstractions", 0.035);
    c.e(0.3, "\r\n");
    c.e(1.2, &format!("{gray}thinking\u{2026}{reset}\r\n\r\n"));
    c.e(0.5, &format!("{green}Garudust{reset}  High-level code blooms \u{2014}\r\n"));
    c.e(0.4, "           no runtime cost beneath it,\r\n");
    c.e(0.4, "           iron runs as thought.\r\n");

    c.e(1.5, &format!("\r\n{dim}^C{reset}\r\n"));
    c.nl();

    let n = c.events.len();
    let duration = c.t;
    fs::write(&out_path, c.render()).expect("failed to write cast file");
    eprintln!("wrote {n} events \u{2192} {}  ({duration:.1}s)", out_path.display());
}
