use std::io::{self, Write};
use std::path::Path;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute, queue,
    style::{Attribute, Print, SetAttribute},
    terminal::{self, ClearType},
};
use garudust_core::config::AgentConfig;

const PLATFORMS: &[(&str, &[(&str, &str)])] = &[
    ("Telegram", &[("Telegram bot token", "TELEGRAM_TOKEN")]),
    ("Discord", &[("Discord bot token", "DISCORD_TOKEN")]),
    (
        "Slack",
        &[
            ("Slack bot token (xoxb-...)", "SLACK_BOT_TOKEN"),
            ("Slack app token (xapp-...)", "SLACK_APP_TOKEN"),
        ],
    ),
    (
        "Matrix",
        &[
            ("Matrix homeserver URL", "MATRIX_HOMESERVER"),
            ("Matrix user (@bot:example.com)", "MATRIX_USER"),
            ("Matrix password", "MATRIX_PASSWORD"),
        ],
    ),
    (
        "LINE",
        &[
            ("LINE channel access token", "LINE_CHANNEL_TOKEN"),
            ("LINE channel secret", "LINE_CHANNEL_SECRET"),
        ],
    ),
];

pub async fn run() -> anyhow::Result<()> {
    let home_dir = AgentConfig::garudust_dir();
    std::fs::create_dir_all(&home_dir)?;

    let existing = AgentConfig::load();
    let is_reconfigure = home_dir.join("config.yaml").exists();

    println!("Garudust Setup");
    println!("{}", "─".repeat(48));
    if is_reconfigure {
        println!("Existing configuration found.");
        println!("Press Enter to keep the current value, or type a new one.\n");
    } else {
        println!("Press Enter to accept the [default] value.\n");
    }

    // ── Mode ──────────────────────────────────────────────────────────────────
    println!("Setup mode:");
    println!("  1) Quick — provider + model only");
    println!("  2) Full  — provider, model, and platform adapters");
    let mode = prompt("Choose mode", Some("1"));
    let full = matches!(mode.trim(), "2" | "full");
    println!();

    // ── Provider ──────────────────────────────────────────────────────────────
    let ollama_detected = std::net::TcpStream::connect("127.0.0.1:11434").is_ok();
    let ollama_hint = if ollama_detected { " ✓ detected" } else { "" };

    let current_num = if is_reconfigure {
        match existing.provider.as_str() {
            "openrouter" => "2",
            "anthropic" => "3",
            "vllm" => "4",
            "custom" => "5",
            _ => "1",
        }
    } else {
        "1"
    };

    println!("LLM Provider:");
    println!("  1) ollama      — local Ollama, no API key needed{ollama_hint}");
    println!("  2) openrouter  — 200+ hosted models (openrouter.ai)");
    println!("  3) anthropic   — Claude directly");
    println!("  4) vllm        — self-hosted vLLM server");
    println!("  5) custom      — any OpenAI-compatible endpoint");
    let choice = prompt("Choose provider", Some(current_num));
    let provider = match choice.trim() {
        "2" | "openrouter" => "openrouter",
        "3" | "anthropic" => "anthropic",
        "4" | "vllm" => "vllm",
        "5" | "custom" => "custom",
        _ => "ollama",
    };
    println!();

    // ── Credentials / endpoint ────────────────────────────────────────────────
    let mut env_vars: Vec<(&'static str, String)> = Vec::new();
    let mut custom_base_url: Option<String> = None;

    match provider {
        "anthropic" => {
            let cur = read_env_file(&home_dir, "ANTHROPIC_API_KEY");
            if let Some(v) = prompt_secret("ANTHROPIC_API_KEY", cur.as_deref())? {
                env_vars.push(("ANTHROPIC_API_KEY", v));
            }
        }
        "vllm" => {
            let cur_url = read_env_file(&home_dir, "VLLM_BASE_URL")
                .unwrap_or_else(|| "http://localhost:8000/v1".into());
            let url = prompt("VLLM_BASE_URL", Some(&cur_url));
            let url = if url.is_empty() { cur_url } else { url };
            env_vars.push(("VLLM_BASE_URL", url));

            let cur_key = read_env_file(&home_dir, "VLLM_API_KEY");
            if let Some(v) = prompt_secret("VLLM_API_KEY (Enter to skip)", cur_key.as_deref())? {
                env_vars.push(("VLLM_API_KEY", v));
            }
        }
        "ollama" => {
            let cur_url = read_env_file(&home_dir, "OLLAMA_BASE_URL")
                .unwrap_or_else(|| "http://localhost:11434".into());
            let url = prompt("OLLAMA_BASE_URL", Some(&cur_url));
            let url = if url.is_empty() { cur_url } else { url };
            env_vars.push(("OLLAMA_BASE_URL", url));
        }
        "custom" => {
            let cur_url = existing.base_url.as_deref();
            let url = prompt("Base URL (e.g. http://localhost:8000/v1)", cur_url);
            if !url.is_empty() {
                custom_base_url = Some(url);
            } else if let Some(u) = existing.base_url.clone() {
                custom_base_url = Some(u);
            }
            let cur_key = read_env_file(&home_dir, "OPENROUTER_API_KEY");
            if let Some(v) = prompt_secret("API key (Enter to skip)", cur_key.as_deref())? {
                env_vars.push(("OPENROUTER_API_KEY", v));
            }
        }
        _ => {
            let cur = read_env_file(&home_dir, "OPENROUTER_API_KEY");
            if let Some(v) = prompt_secret("OPENROUTER_API_KEY", cur.as_deref())? {
                env_vars.push(("OPENROUTER_API_KEY", v));
            }
        }
    }
    println!();

    // ── Model ─────────────────────────────────────────────────────────────────
    let default_model = if is_reconfigure && provider == existing.provider {
        existing.model.as_str()
    } else {
        match provider {
            "ollama" => "llama3.2",
            "anthropic" => "claude-sonnet-4-6",
            "openrouter" => "anthropic/claude-sonnet-4-6",
            _ => "",
        }
    };
    let model_input = prompt(
        "Model",
        if default_model.is_empty() {
            None
        } else {
            Some(default_model)
        },
    );
    let model = if model_input.is_empty() {
        default_model.to_string()
    } else {
        model_input
    };
    println!();

    // ── Optional tools + platform adapters (Full mode) ───────────────────────
    if full {
        println!("Optional Tools (Enter to keep current / skip):");
        let cur_brave = read_env_file(&home_dir, "BRAVE_SEARCH_API_KEY");
        if let Some(v) = prompt_secret(
            "Brave Search API key (web_search tool)",
            cur_brave.as_deref(),
        )? {
            env_vars.push(("BRAVE_SEARCH_API_KEY", v));
        }
        println!();

        // Pre-tick platforms that already have at least one token in .env
        let preselected: Vec<bool> = PLATFORMS
            .iter()
            .map(|(_, fields)| {
                fields
                    .iter()
                    .any(|(_, var)| read_env_file(&home_dir, var).is_some())
            })
            .collect();

        println!("Platform Adapters:");
        println!("  ↑↓ to move  ·  Space to select  ·  Enter to confirm\n");

        let names: Vec<&str> = PLATFORMS.iter().map(|(name, _)| *name).collect();
        let selected = multi_select(&names, &preselected)?;
        println!();

        for (i, (_, fields)) in PLATFORMS.iter().enumerate() {
            if !selected[i] {
                continue;
            }
            for (label, var) in *fields {
                let cur = read_env_file(&home_dir, var);
                if let Some(v) = prompt_secret(label, cur.as_deref())? {
                    env_vars.push((var, v));
                }
            }
        }
        println!();
    }

    // ── Persist ───────────────────────────────────────────────────────────────
    for (var, val) in &env_vars {
        AgentConfig::set_env_var(&home_dir, var, val)?;
    }

    let mut new_config = AgentConfig {
        home_dir: home_dir.clone(),
        provider: provider.to_string(),
        model,
        base_url: custom_base_url,
        ..AgentConfig::default()
    };
    new_config.save_yaml()?;

    println!("Configuration saved to {}", home_dir.display());
    println!();

    // ── Doctor ────────────────────────────────────────────────────────────────
    let api_key = env_vars
        .iter()
        .find(|(v, _)| {
            matches!(
                *v,
                "ANTHROPIC_API_KEY" | "OPENROUTER_API_KEY" | "VLLM_API_KEY"
            )
        })
        .map(|(_, k)| k.clone())
        .or(existing.api_key);
    if let Some(key) = api_key {
        new_config.api_key = Some(key);
    }
    super::doctor::run(&new_config).await;

    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Read a raw value from ~/.garudust/.env without going through the OnceLock cache.
fn read_env_file(home_dir: &Path, key: &str) -> Option<String> {
    let content = std::fs::read_to_string(home_dir.join(".env")).ok()?;
    let prefix = format!("{key}=");
    for line in content.lines() {
        if let Some(val) = line.trim().strip_prefix(&prefix) {
            let val = val.trim().trim_matches('"').trim_matches('\'');
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}

/// Mask a secret: `sk-ant-api03-abcdef…xyz` → `sk-an••••wxyz`.
fn mask_secret(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() < 8 {
        return "••••".to_string();
    }
    let prefix: String = chars.iter().take(4).collect();
    let suffix: String = chars
        .iter()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("{prefix}••••{suffix}")
}

/// Prompt for a potentially-sensitive value.
/// Shows `[current: ••••]` when an existing value is present.
/// Returns `None` (keep existing) if the user presses Enter with no input.
/// Returns `Some(new_value)` when the user types a new value.
fn prompt_secret(label: &str, existing: Option<&str>) -> anyhow::Result<Option<String>> {
    if let Some(cur) = existing {
        print!("  {label} [current: {}]: ", mask_secret(cur));
    } else {
        print!("  {label}: ");
    }
    io::stdout().flush()?;

    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    let trimmed = buf.trim().to_string();

    Ok(if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    })
}

/// Prompt for a non-secret value. Shows `[default]` in brackets.
/// Returns the default if the user presses Enter with no input.
fn prompt(label: &str, default: Option<&str>) -> String {
    match default {
        Some(d) if !d.is_empty() => print!("  {label} [{d}]: "),
        _ => print!("  {label}: "),
    }
    io::stdout().flush().ok();

    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap_or(0);
    let trimmed = buf.trim().to_string();

    if trimmed.is_empty() {
        default.unwrap_or("").to_string()
    } else {
        trimmed
    }
}

/// Render an interactive checkbox list. `preselected` sets the initial state.
/// Returns a bool vec (same length as `items`) indicating which are selected.
fn multi_select(items: &[&str], preselected: &[bool]) -> anyhow::Result<Vec<bool>> {
    let mut selected = preselected.to_vec();
    selected.resize(items.len(), false);
    let mut cursor_pos: usize = 0;
    let mut stdout = io::stdout();

    terminal::enable_raw_mode()?;
    execute!(stdout, cursor::Hide)?;
    draw_checkboxes(&mut stdout, items, &selected, cursor_pos)?;

    loop {
        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
            match code {
                KeyCode::Up | KeyCode::Char('k') => {
                    cursor_pos = cursor_pos.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') if cursor_pos + 1 < items.len() => {
                    cursor_pos += 1;
                }
                KeyCode::Char(' ') => {
                    selected[cursor_pos] = !selected[cursor_pos];
                }
                KeyCode::Enter => break,
                KeyCode::Char('q') | KeyCode::Esc => {
                    selected.fill(false);
                    break;
                }
                _ => {}
            }
            draw_checkboxes(&mut stdout, items, &selected, cursor_pos)?;
        }
    }

    terminal::disable_raw_mode()?;
    execute!(stdout, cursor::Show)?;
    writeln!(stdout)?;

    Ok(selected)
}

fn draw_checkboxes(
    stdout: &mut io::Stdout,
    items: &[&str],
    selected: &[bool],
    cursor_pos: usize,
) -> anyhow::Result<()> {
    if items.len() > 1 {
        queue!(
            stdout,
            cursor::MoveUp(u16::try_from(items.len() - 1).unwrap_or(u16::MAX)),
            cursor::MoveToColumn(0),
        )?;
    } else {
        queue!(stdout, cursor::MoveToColumn(0))?;
    }

    for (i, item) in items.iter().enumerate() {
        let checkbox = if selected[i] { "[✓]" } else { "[ ]" };
        queue!(stdout, terminal::Clear(ClearType::CurrentLine))?;

        if i == cursor_pos {
            queue!(
                stdout,
                SetAttribute(Attribute::Bold),
                Print(format!("  {checkbox} {item}")),
                SetAttribute(Attribute::Reset),
            )?;
        } else {
            queue!(stdout, Print(format!("  {checkbox} {item}")))?;
        }

        if i + 1 < items.len() {
            queue!(stdout, Print("\r\n"))?;
        }
    }

    stdout.flush()?;
    Ok(())
}
