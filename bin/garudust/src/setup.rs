use std::io::{self, Write};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute, queue,
    style::{Attribute, Print, SetAttribute},
    terminal::{self, ClearType},
};
use garudust_core::config::AgentConfig;

pub async fn run() -> anyhow::Result<()> {
    let home_dir = AgentConfig::garudust_dir();
    std::fs::create_dir_all(&home_dir)?;

    println!("Garudust Setup");
    println!("{}", "─".repeat(48));
    println!("Press Enter to accept the [default] value.\n");

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
    println!("LLM Provider:");
    println!("  1) ollama      — local Ollama, no API key needed{ollama_hint}");
    println!("  2) openrouter  — 200+ hosted models (openrouter.ai)");
    println!("  3) anthropic   — Claude directly");
    println!("  4) vllm        — self-hosted vLLM server");
    println!("  5) custom      — any OpenAI-compatible endpoint");
    let choice = prompt("Choose provider", Some("1"));
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
            let k = prompt("ANTHROPIC_API_KEY", None);
            if !k.is_empty() {
                env_vars.push(("ANTHROPIC_API_KEY", k));
            }
        }
        "vllm" => {
            let url = prompt("VLLM_BASE_URL", Some("http://localhost:8000/v1"));
            let url = if url.is_empty() {
                "http://localhost:8000/v1".to_string()
            } else {
                url
            };
            env_vars.push(("VLLM_BASE_URL", url));
            let k = prompt("VLLM_API_KEY (Enter to skip)", Some(""));
            if !k.is_empty() {
                env_vars.push(("VLLM_API_KEY", k));
            }
        }
        "ollama" => {
            let url = prompt("OLLAMA_BASE_URL", Some("http://localhost:11434"));
            let url = if url.is_empty() {
                "http://localhost:11434".to_string()
            } else {
                url
            };
            env_vars.push(("OLLAMA_BASE_URL", url));
        }
        "custom" => {
            let url = prompt("Base URL (e.g. http://localhost:8000/v1)", None);
            if !url.is_empty() {
                custom_base_url = Some(url);
            }
            let k = prompt("API key (Enter to skip)", Some(""));
            if !k.is_empty() {
                env_vars.push(("OPENROUTER_API_KEY", k));
            }
        }
        _ => {
            let k = prompt("OPENROUTER_API_KEY", None);
            if !k.is_empty() {
                env_vars.push(("OPENROUTER_API_KEY", k));
            }
        }
    }
    println!();

    // ── Model ─────────────────────────────────────────────────────────────────
    let default_model = match provider {
        "ollama" => "llama3.2",
        "anthropic" => "claude-sonnet-4-6",
        "openrouter" => "anthropic/claude-sonnet-4-6",
        _ => "",
    };
    let hint = if default_model.is_empty() {
        None
    } else {
        Some(default_model)
    };
    let model_input = prompt("Model", hint);
    let model = if model_input.is_empty() {
        default_model.to_string()
    } else {
        model_input
    };
    println!();

    // ── Optional tools + platform adapters (Full mode) ───────────────────────
    if full {
        println!("Optional Tools (Enter to skip each):");
        let tool_fields: &[(&str, &str)] = &[(
            "Brave Search API key (web_search tool)",
            "BRAVE_SEARCH_API_KEY",
        )];
        for (label, var) in tool_fields {
            let val = prompt(label, Some(""));
            if !val.is_empty() {
                env_vars.push((var, val));
            }
        }
        println!();

        // ── Platform selection via checkbox menu ──────────────────────────────
        let platforms: &[(&str, &str)] = &[
            ("Telegram", "telegram"),
            ("Discord", "discord"),
            ("Slack", "slack"),
            ("Matrix", "matrix"),
            ("LINE", "line"),
        ];

        println!("Platform Adapters:");
        println!("  ↑↓ to move  ·  Space to select  ·  Enter to confirm\n");

        let names: Vec<&str> = platforms.iter().map(|(name, _)| *name).collect();
        let selected = multi_select(&names)?;
        println!();

        // ── Per-platform config fields ────────────────────────────────────────
        for (i, (_name, id)) in platforms.iter().enumerate() {
            if !selected[i] {
                continue;
            }

            let fields: &[(&str, &str)] = match *id {
                "telegram" => &[("Telegram bot token", "TELEGRAM_TOKEN")],
                "discord" => &[("Discord bot token", "DISCORD_TOKEN")],
                "slack" => &[
                    ("Slack bot token (xoxb-...)", "SLACK_BOT_TOKEN"),
                    ("Slack app token (xapp-...)", "SLACK_APP_TOKEN"),
                ],
                "matrix" => &[
                    ("Matrix homeserver URL", "MATRIX_HOMESERVER"),
                    ("Matrix user (@bot:example.com)", "MATRIX_USER"),
                    ("Matrix password", "MATRIX_PASSWORD"),
                ],
                "line" => &[
                    ("LINE channel access token", "LINE_CHANNEL_TOKEN"),
                    ("LINE channel secret", "LINE_CHANNEL_SECRET"),
                ],
                _ => &[],
            };

            for (label, var) in fields {
                let val = prompt(label, Some(""));
                if !val.is_empty() {
                    env_vars.push((var, val));
                }
            }
        }
        println!();
    }

    // ── Persist ───────────────────────────────────────────────────────────────
    for (var, val) in &env_vars {
        AgentConfig::set_env_var(&home_dir, var, val)?;
    }

    let mut config = AgentConfig {
        home_dir: home_dir.clone(),
        provider: provider.to_string(),
        model,
        base_url: custom_base_url,
        ..AgentConfig::default()
    };
    config.save_yaml()?;

    println!("Configuration saved to {}", home_dir.display());
    println!();

    // ── Doctor ────────────────────────────────────────────────────────────────
    if let Some((_, key)) = env_vars.iter().find(|(v, _)| {
        matches!(
            *v,
            "ANTHROPIC_API_KEY" | "OPENROUTER_API_KEY" | "VLLM_API_KEY"
        )
    }) {
        config.api_key = Some(key.clone());
    }
    super::doctor::run(&config).await;

    Ok(())
}

/// Render an interactive checkbox list. Returns a bool vec (same length as
/// `items`) indicating which entries the user selected.
fn multi_select(items: &[&str]) -> anyhow::Result<Vec<bool>> {
    let mut selected = vec![false; items.len()];
    let mut cursor_pos: usize = 0;
    let mut stdout = io::stdout();

    terminal::enable_raw_mode()?;

    // Hide cursor while navigating
    execute!(stdout, cursor::Hide)?;

    // Draw initial list
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
                    // Deselect all on quit
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

    // Move past the drawn list
    writeln!(stdout)?;

    Ok(selected)
}

fn draw_checkboxes(
    stdout: &mut io::Stdout,
    items: &[&str],
    selected: &[bool],
    cursor_pos: usize,
) -> anyhow::Result<()> {
    // Move up to redraw from the top of the list
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

fn prompt(label: &str, default: Option<&str>) -> String {
    if let Some(d) = default {
        if d.is_empty() {
            print!("  {label}: ");
        } else {
            print!("  {label} [{d}]: ");
        }
    } else {
        print!("  {label}: ");
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
