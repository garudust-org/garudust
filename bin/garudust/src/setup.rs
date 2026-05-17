use std::io::{self, Write};
use std::path::Path;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute, queue,
    style::{Attribute, Print, SetAttribute},
    terminal::{self, ClearType},
};
use garudust_core::config::{AgentConfig, ProviderProfile, WebhookPlatformConfig};

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
    (
        "WhatsApp",
        &[
            ("WhatsApp access token", "WHATSAPP_ACCESS_TOKEN"),
            ("WhatsApp phone number ID", "WHATSAPP_PHONE_NUMBER_ID"),
            (
                "WhatsApp app secret (for signature verification)",
                "WHATSAPP_APP_SECRET",
            ),
            (
                "WhatsApp verify token (for webhook setup)",
                "WHATSAPP_VERIFY_TOKEN",
            ),
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
            "thaillm" => "5",
            "custom" => "6",
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
    println!("  5) thaillm     — ThaiLLM (NSTDA, thaillm.or.th)");
    println!("  6) custom      — any OpenAI-compatible endpoint");
    let choice = prompt("Choose provider", Some(current_num));
    let provider = match choice.trim() {
        "2" | "openrouter" => "openrouter",
        "3" | "anthropic" => "anthropic",
        "4" | "vllm" => "vllm",
        "5" | "thaillm" => "thaillm",
        "6" | "custom" => "custom",
        _ => "ollama",
    };
    println!();

    // ── Credentials / endpoint ────────────────────────────────────────────────
    // Remove legacy base-URL env vars — provider/base_url now live in config.yaml.
    // Also clear stale entries for providers not selected so they can't override.
    for var in &["VLLM_BASE_URL", "OLLAMA_BASE_URL", "GARUDUST_MODEL"] {
        remove_env_var(&home_dir, var)?;
    }

    let mut env_vars: Vec<(&'static str, String)> = Vec::new();
    let mut custom_base_url: Option<String> = None;

    match provider {
        "anthropic" => {
            let cur = read_env_file(&home_dir, "ANTHROPIC_API_KEY");
            if let Some(v) =
                prompt_secret("ANTHROPIC_API_KEY", "ANTHROPIC_API_KEY", cur.as_deref())?
            {
                env_vars.push(("ANTHROPIC_API_KEY", v));
            }
        }
        "vllm" => {
            let cur_url = existing
                .base_url
                .as_deref()
                .unwrap_or("http://localhost:8000/v1");
            let url = prompt("base_url (vLLM server)", Some(cur_url));
            let url = if url.is_empty() {
                cur_url.to_string()
            } else {
                url
            };
            custom_base_url = Some(url);

            let cur_key = read_env_file(&home_dir, "VLLM_API_KEY");
            if let Some(v) = prompt_secret(
                "VLLM_API_KEY",
                "VLLM_API_KEY (Enter to skip)",
                cur_key.as_deref(),
            )? {
                env_vars.push(("VLLM_API_KEY", v));
            }
        }
        "thaillm" => {
            let cur = read_env_file(&home_dir, "THAILLM_API_KEY");
            if let Some(v) = prompt_secret("THAILLM_API_KEY", "THAILLM_API_KEY", cur.as_deref())? {
                env_vars.push(("THAILLM_API_KEY", v));
            }
        }
        "ollama" => {
            let cur_url = existing
                .base_url
                .as_deref()
                .unwrap_or("http://localhost:11434");
            let url = prompt("base_url (Ollama server)", Some(cur_url));
            let url = if url.is_empty() {
                cur_url.to_string()
            } else {
                url
            };
            custom_base_url = Some(url);
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
            if let Some(v) = prompt_secret(
                "OPENROUTER_API_KEY",
                "API key (Enter to skip)",
                cur_key.as_deref(),
            )? {
                env_vars.push(("OPENROUTER_API_KEY", v));
            }
        }
        _ => {
            let cur = read_env_file(&home_dir, "OPENROUTER_API_KEY");
            if let Some(v) =
                prompt_secret("OPENROUTER_API_KEY", "OPENROUTER_API_KEY", cur.as_deref())?
            {
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
            "thaillm" => "typhoon-s-thaillm-8b-instruct",
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
    // Tracks LINE/WhatsApp selections so the post-yaml-load step can flip
    // `enabled` on the corresponding `platforms.*` block. `None` in Quick mode
    // means "don't touch the yaml's platforms section at all".
    let mut webhook_platform_selections: Option<Vec<(&'static str, bool)>> = None;
    let mut custom_profiles: Vec<(String, ProviderProfile)> = Vec::new();
    if full {
        println!("Optional Tools (Enter to keep current / skip):");
        let cur_brave = read_env_file(&home_dir, "BRAVE_SEARCH_API_KEY");
        if let Some(v) = prompt_secret(
            "BRAVE_SEARCH_API_KEY",
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

        webhook_platform_selections = Some(
            PLATFORMS
                .iter()
                .zip(selected.iter())
                .map(|((name, _), sel)| (*name, *sel))
                .collect(),
        );

        for (i, (_, fields)) in PLATFORMS.iter().enumerate() {
            if !selected[i] {
                continue;
            }
            for (label, var) in *fields {
                let cur = read_env_file(&home_dir, var);
                if let Some(v) = prompt_secret(var, label, cur.as_deref())? {
                    env_vars.push((var, v));
                }
            }
        }
        println!();

        // ── Custom Provider Profiles ──────────────────────────────────────────
        println!("Custom Provider Profiles (optional — for routing: or tool model overrides):");
        println!("  Leave profile name blank to skip.\n");
        loop {
            let alias = prompt("Profile alias (e.g. groq-backup, local)", None);
            if alias.is_empty() {
                break;
            }
            let provider_name = prompt(
                "Provider name (e.g. groq, openai) or leave blank for custom URL",
                None,
            );
            let url_input = if provider_name.is_empty() {
                prompt("Base URL", None)
            } else {
                prompt("Base URL (Enter to use provider default)", None)
            };
            let key_env = prompt("API key env var (e.g. GROQ_API_KEY_2)", None);
            let model_input = prompt("Default model for this profile (Enter to skip)", None);

            let profile = ProviderProfile {
                name: if provider_name.is_empty() {
                    None
                } else {
                    Some(provider_name)
                },
                url: if url_input.is_empty() {
                    None
                } else {
                    Some(url_input)
                },
                key: if key_env.is_empty() {
                    None
                } else {
                    Some(format!("${{{key_env}}}"))
                },
                model: if model_input.is_empty() {
                    None
                } else {
                    Some(model_input)
                },
            };
            custom_profiles.push((alias, profile));
            println!();
        }
        println!();
    }

    // ── Persist ───────────────────────────────────────────────────────────────
    for (var, val) in &env_vars {
        AgentConfig::set_env_var(&home_dir, var, val)?;
    }

    // Preserve existing YAML settings (compression, context_window, disabled_toolsets, etc.)
    // and only overwrite the fields that setup controls.
    let yaml_path = home_dir.join("config.yaml");
    let mut new_config: AgentConfig = if yaml_path.exists() {
        let src = std::fs::read_to_string(&yaml_path).unwrap_or_default();
        serde_yaml::from_str(&src).unwrap_or_default()
    } else {
        AgentConfig::default()
    };
    new_config.home_dir = home_dir.clone();
    new_config.provider = provider.to_string();
    new_config.model = model;
    new_config.base_url = custom_base_url;

    // Sync webhook-based platform adapters (LINE, WhatsApp) into yaml. Tokens
    // live in .env; this block controls whether the adapter starts and on which
    // port/path. Existing port/webhook_path is preserved when present.
    if let Some(selections) = webhook_platform_selections {
        for (name, is_selected) in selections {
            match name {
                "LINE" => sync_platform(
                    &mut new_config.platforms.line,
                    is_selected,
                    WebhookPlatformConfig::default_line,
                ),
                "WhatsApp" => sync_platform(
                    &mut new_config.platforms.whatsapp,
                    is_selected,
                    WebhookPlatformConfig::default_whatsapp,
                ),
                _ => {} // Telegram/Discord/Slack/Matrix don't use webhook config
            }
        }
    }

    for (alias, profile) in custom_profiles {
        new_config.providers.insert(alias, profile);
    }

    new_config.save_yaml()?;

    println!("Configuration saved to {}", home_dir.display());
    println!();

    // ── Doctor ────────────────────────────────────────────────────────────────
    let api_key = env_vars
        .iter()
        .find(|(v, _)| {
            matches!(
                *v,
                "ANTHROPIC_API_KEY" | "OPENROUTER_API_KEY" | "VLLM_API_KEY" | "THAILLM_API_KEY"
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

/// Sync a webhook-based platform's yaml block with the wizard's selection.
/// - Selected + block exists → flip `enabled = true`, keep port/path.
/// - Selected + no block → create one from `make_default`.
/// - Unselected + block exists → flip `enabled = false`, keep port/path so a
///   later re-enable preserves the user's customizations.
/// - Unselected + no block → no-op.
fn sync_platform(
    slot: &mut Option<WebhookPlatformConfig>,
    is_selected: bool,
    make_default: fn() -> WebhookPlatformConfig,
) {
    match (is_selected, slot.as_mut()) {
        (true, Some(cfg)) => cfg.enabled = true,
        (true, None) => *slot = Some(make_default()),
        (false, Some(cfg)) => cfg.enabled = false,
        (false, None) => {}
    }
}

/// Validate a token value against known format rules.
/// Returns an error hint string when the value looks wrong, or `None` when OK.
fn validate_token(var: &str, val: &str) -> Option<&'static str> {
    match var {
        "ANTHROPIC_API_KEY" if !val.starts_with("sk-ant-") => {
            return Some("expected format: sk-ant-… (starts with 'sk-ant-')");
        }
        "OPENROUTER_API_KEY" if !val.starts_with("sk-or-") => {
            return Some("expected format: sk-or-… (starts with 'sk-or-')");
        }
        "TELEGRAM_TOKEN" => {
            let mut parts = val.splitn(2, ':');
            let digits_ok = parts
                .next()
                .is_some_and(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
            let suffix_ok = parts.next().is_some_and(|p| p.len() >= 30);
            if !digits_ok || !suffix_ok {
                return Some("expected format: 123456789:AAFxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");
            }
        }
        "DISCORD_TOKEN" if val.split('.').count() != 3 || val.len() < 50 => {
            return Some("expected format: three Base64 segments separated by '.' (~70 chars)");
        }
        "SLACK_BOT_TOKEN" if !val.starts_with("xoxb-") => {
            return Some("expected format: xoxb-… (starts with 'xoxb-')");
        }
        "SLACK_APP_TOKEN" if !val.starts_with("xapp-") => {
            return Some("expected format: xapp-… (starts with 'xapp-')");
        }
        "MATRIX_HOMESERVER" if !val.starts_with("https://") && !val.starts_with("http://") => {
            return Some("expected format: https://matrix.example.com");
        }
        "MATRIX_USER" if !val.starts_with('@') || !val.contains(':') => {
            return Some("expected format: @username:server.com");
        }
        "LINE_CHANNEL_TOKEN" if val.len() < 20 => {
            return Some("expected: non-empty string, at least 20 characters");
        }
        "LINE_CHANNEL_SECRET" if val.len() != 32 || !val.chars().all(|c| c.is_ascii_hexdigit()) => {
            return Some("expected format: 32-character hex string");
        }
        "WHATSAPP_PHONE_NUMBER_ID" if !val.chars().all(|c| c.is_ascii_digit()) => {
            return Some("expected format: numeric ID (e.g. 123456789012345)");
        }
        _ => {}
    }
    None
}

/// Remove a key from ~/.garudust/.env if present.
fn remove_env_var(home_dir: &Path, key: &str) -> std::io::Result<()> {
    let env_path = home_dir.join(".env");
    if !env_path.exists() {
        return Ok(());
    }
    let content = std::fs::read_to_string(&env_path)?;
    let prefix = format!("{key}=");
    let filtered: Vec<&str> = content
        .lines()
        .filter(|l| !l.trim().starts_with(prefix.as_str()))
        .collect();
    std::fs::write(&env_path, filtered.join("\n") + "\n")
}

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

/// Prompt for a potentially-sensitive value, with one-time format validation.
/// Shows `[current: ••••]` when an existing value is present.
/// Returns `None` (keep existing) if the user presses Enter with no input.
/// Returns `Some(new_value)` when the user types a new value.
/// On a format mismatch: warns inline and re-prompts once; accepts whatever the
/// user enters on the second attempt so valid non-standard tokens still work.
fn prompt_secret(var: &str, label: &str, existing: Option<&str>) -> anyhow::Result<Option<String>> {
    let read_line = || -> anyhow::Result<String> {
        let mut buf = String::new();
        io::stdin().read_line(&mut buf)?;
        Ok(buf.trim().to_string())
    };

    if let Some(cur) = existing {
        print!("  {label} [current: {}]: ", mask_secret(cur));
    } else {
        print!("  {label}: ");
    }
    io::stdout().flush()?;

    let first = read_line()?;
    if first.is_empty() {
        return Ok(None);
    }

    if let Some(hint) = validate_token(var, &first) {
        println!("  ✗ {hint}");
        print!("  {label} (press Enter to use as-is): ");
        io::stdout().flush()?;
        let second = read_line()?;
        return Ok(Some(if second.is_empty() { first } else { second }));
    }

    Ok(Some(first))
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

#[cfg(test)]
mod tests {
    use super::validate_token;

    #[test]
    fn anthropic_key_valid() {
        assert!(validate_token("ANTHROPIC_API_KEY", "sk-ant-api03-abc").is_none());
    }

    #[test]
    fn anthropic_key_invalid() {
        assert!(validate_token("ANTHROPIC_API_KEY", "sk-abc-wrongprefix").is_some());
    }

    #[test]
    fn openrouter_key_valid() {
        assert!(validate_token("OPENROUTER_API_KEY", "sk-or-v1-abc123").is_none());
    }

    #[test]
    fn openrouter_key_invalid() {
        assert!(validate_token("OPENROUTER_API_KEY", "sk-ant-abc").is_some());
    }

    #[test]
    fn telegram_token_valid() {
        assert!(validate_token(
            "TELEGRAM_TOKEN",
            "123456789:AAFabcdefghijklmnopqrstuvwxyz012"
        )
        .is_none());
    }

    #[test]
    fn telegram_token_invalid_no_colon() {
        assert!(validate_token("TELEGRAM_TOKEN", "123456789AAFabc").is_some());
    }

    #[test]
    fn telegram_token_invalid_non_digit_id() {
        assert!(
            validate_token("TELEGRAM_TOKEN", "abcde:AAFabcdefghijklmnopqrstuvwxyz012").is_some()
        );
    }

    #[test]
    fn slack_bot_token_valid() {
        assert!(validate_token("SLACK_BOT_TOKEN", "xoxb-123-abc").is_none());
    }

    #[test]
    fn slack_bot_token_invalid() {
        assert!(validate_token("SLACK_BOT_TOKEN", "xoxp-123-abc").is_some());
    }

    #[test]
    fn slack_app_token_valid() {
        assert!(validate_token("SLACK_APP_TOKEN", "xapp-1-abc").is_none());
    }

    #[test]
    fn matrix_homeserver_valid() {
        assert!(validate_token("MATRIX_HOMESERVER", "https://matrix.example.com").is_none());
    }

    #[test]
    fn matrix_homeserver_invalid() {
        assert!(validate_token("MATRIX_HOMESERVER", "matrix.example.com").is_some());
    }

    #[test]
    fn matrix_user_valid() {
        assert!(validate_token("MATRIX_USER", "@bot:example.com").is_none());
    }

    #[test]
    fn matrix_user_invalid() {
        assert!(validate_token("MATRIX_USER", "bot_example_com").is_some());
    }

    #[test]
    fn line_channel_token_valid() {
        assert!(validate_token("LINE_CHANNEL_TOKEN", "a".repeat(20).as_str()).is_none());
    }

    #[test]
    fn line_channel_token_too_short() {
        assert!(validate_token("LINE_CHANNEL_TOKEN", "short").is_some());
    }

    #[test]
    fn line_channel_secret_valid() {
        assert!(
            validate_token("LINE_CHANNEL_SECRET", "abcdef1234567890abcdef1234567890").is_none()
        );
    }

    #[test]
    fn line_channel_secret_invalid_length() {
        assert!(validate_token("LINE_CHANNEL_SECRET", "tooshort").is_some());
    }

    #[test]
    fn line_channel_secret_invalid_non_hex() {
        assert!(
            validate_token("LINE_CHANNEL_SECRET", "zbcdef1234567890abcdef123456789z").is_some()
        );
    }

    #[test]
    fn unknown_var_always_passes() {
        assert!(validate_token("SOME_UNKNOWN_VAR", "anything").is_none());
    }

    #[test]
    fn discord_token_valid() {
        let token = "MTIzNDU2Nzg5.ABCDEF.ghijklmnopqrstuvwxyz1234567890abcdefghij";
        assert!(validate_token("DISCORD_TOKEN", token).is_none());
    }

    #[test]
    fn discord_token_invalid_segments() {
        assert!(validate_token("DISCORD_TOKEN", "only.two").is_some());
    }
}
